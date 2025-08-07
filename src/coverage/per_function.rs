use super::{BlockCoverage, CoverageFeedback, CoverageMetric, EdgeCoverage};
use log::{info, warn};
use md5::{compute, Digest};
use path_reduction::json_parser::parse_json_file;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

type BlockID = u32;

/// Per-function path coverage.
pub struct PerFunctionPathCoverage {
    block_cov: BlockCoverage,
    edge_cov: EdgeCoverage,
    coverage: FxHashMap<BlockID, FxHashSet<Digest>>,
    first_to_lasts: FxHashMap<BlockID, FxHashSet<BlockID>>,
    // invariant: total_cov is the sum of
    // the sizes of the coverage sets for each function
    total_cov: usize,
}

impl PerFunctionPathCoverage {
    pub fn from_json(path: &str) -> Self {
        let modules = parse_json_file(path).unwrap();
        let first_to_lasts = modules
            .iter()
            .flat_map(|module| {
                module
                    .functions
                    .iter()
                    .map(|func| (func.entry_block, func.exit_blocks.iter().cloned().collect()))
            })
            .collect();
        Self {
            block_cov: BlockCoverage::default(),
            edge_cov: EdgeCoverage::default(),
            coverage: FxHashMap::default(),
            first_to_lasts,
            total_cov: 0,
        }
    }

    /// Computes the hash of a path
    fn compute_hash(&self, path: &[u32]) -> Digest {
        let bytes: Vec<u8> = path.iter().flat_map(|&num| num.to_ne_bytes()).collect();
        compute(&bytes)
    }

    /// Computes the hash of a path and updates the coverage set and stats
    fn compute_hash_and_update_cov(&mut self, path: &[u32]) -> bool {
        let path_hash = self.compute_hash(path);
        if path.is_empty() {
            warn!("empty path");
            return false;
        }
        let res = self
            .coverage
            .entry(path[0])
            .or_insert_with(FxHashSet::default)
            .insert(path_hash);
        if res {
            self.total_cov += 1;
        }
        res
    }

    /// Precondition: `path` starts with a function entry block
    fn reduce_fun(&mut self, path: &mut &[u32]) -> bool {
        self.reduce_fun1(path, 2)
    }

    /// Precondition: `path` starts with a function entry block
    fn reduce_fun1(&mut self, path: &mut &[u32], k: usize) -> bool {
        let mut reduced_path: Vec<u32> = Vec::new();
        if cfg!(test) {
            println!("unreduced path: {:?}", path);
        }
        let mut new_cov = false;
        let first = if let Some(&first) = path.first() {
            first
        } else {
            info!("redunce_fun: empty block");
            return false;
        };
        info!("reducing fun starting with {}", first);
        *path = &path[1..];
        reduced_path.push(first);
        let lasts = &self
            .first_to_lasts
            .get(&first)
            .expect(&format!("no entry for first block {}", first))
            .clone();
        // handles the case where the function is a single block
        if lasts.contains(&first) {
            return self.compute_hash_and_update_cov(&reduced_path);
        }
        let mut loop_stack: FxHashMap<BlockID, (usize, usize)> = FxHashMap::default();
        while !path.is_empty() {
            let new_block = path[0];
            // function call
            if self.first_to_lasts.contains_key(&new_block) {
                reduced_path.push(new_block);
                new_cov = self.reduce_fun1(path, k) || new_cov;
            } else if lasts.contains(&new_block) {
                reduced_path.push(new_block);
                *path = &path[1..];
                if cfg!(test) {
                    println!("reduced path: {:?}", reduced_path);
                    println!("loop_stack: {:?}", loop_stack);
                }
                return self.compute_hash_and_update_cov(&reduced_path);
            } else {
                if let Some((times, last_idx)) = loop_stack.get(&new_block).cloned() {
                    loop_stack.retain(|&_block, (_times, last_idx_)| *last_idx_ <= last_idx);
                    if times < k {
                        loop_stack.insert(new_block, (times + 1, reduced_path.len()));
                        reduced_path.push(new_block);
                        *path = &path[1..];
                    } else {
                        reduced_path.truncate(last_idx + 1);
                        *path = &path[1..];
                    }
                } else {
                    loop_stack.insert(new_block, (1, reduced_path.len()));
                    reduced_path.push(new_block);
                    *path = &path[1..];
                }
            }
        }
        warn!("partial path");
        if cfg!(test) {
            println!("reduced path: {:?}", reduced_path);
            println!("loop_stack: {:?}", loop_stack);
        }
        self.compute_hash_and_update_cov(&reduced_path)
    }

    #[cfg(test)]
    fn empty() -> Self {
        Self {
            block_cov: BlockCoverage::default(),
            edge_cov: EdgeCoverage::default(),
            coverage: FxHashMap::default(),
            first_to_lasts: FxHashMap::default(),
            total_cov: 0,
        }
    }
}

impl Default for PerFunctionPathCoverage {
    fn default() -> Self {
        let cfg_file = std::env::var("CFG_FILE").unwrap_or_default();
        Self::from_json(&cfg_file)
    }
}

impl CoverageMetric for PerFunctionPathCoverage {
    fn update_from_path(&mut self, mut path: &[u32]) -> CoverageFeedback {
        let block_feedback = self.block_cov.update_from_path(&path);
        let edge_feedback = self.edge_cov.update_from_path(&path);
        let mut new_cov = false;
        while !path.is_empty() {
            new_cov = self.reduce_fun(&mut path) || new_cov;
        }
        if block_feedback.new_cov() {
            block_feedback
        } else if edge_feedback.new_cov() {
            edge_feedback
        } else if new_cov {
            CoverageFeedback::NewPath {
                uniqueness: edge_feedback.get_edge_uniqueness(),
            }
        } else {
            CoverageFeedback::NoCoverage(edge_feedback.get_edge_uniqueness())
        }
    }

    fn cov_info(&self) -> serde_json::Value {
        Value::Number(self.total_cov.into())
    }

    fn name(&self) -> &'static str {
        "pfp"
    }

    fn priority(&self) -> usize {
        20
    }
}

mod test {
    use super::PerFunctionPathCoverage;

    #[test]
    fn test1() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (f | 2)
        pfp.first_to_lasts.insert(1, [2].into_iter().collect());
        let path = vec![1, 1, 2, 2];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test1_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (f | 2)
        pfp.first_to_lasts.insert(1, [2].into_iter().collect());
        let path = vec![1, 1, 2, 2];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test2() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (2)* 3
        pfp.first_to_lasts.insert(1, [3].into_iter().collect());
        let path = vec![1, 2, 2, 2, 3];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test2_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (2)* 3
        pfp.first_to_lasts.insert(1, [3].into_iter().collect());
        let path = vec![1, 2, 2, 2, 3];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test3() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 2, 3, 4];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test3_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 2, 3, 4];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test4() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (2(3|4)*)*5
        pfp.first_to_lasts.insert(1, [5].into_iter().collect());
        let path = vec![1, 2, 3, 3, 3, 4, 2, 3, 4, 5];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test4_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (2(3|4)*)*5
        pfp.first_to_lasts.insert(1, [5].into_iter().collect());
        let path = vec![1, 2, 3, 3, 3, 4, 2, 3, 4, 5];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test5() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 2, 3, 2, 3, 4];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test5_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 2, 3, 2, 3, 4];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test6() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23*)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 3, 3, 2, 3, 3, 3, 4];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test6_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23*)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 3, 3, 2, 3, 3, 3, 4];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test7() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23*)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 3, 3, 2, 3, 3, 3, 2, 3, 3, 3, 3, 4];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test7_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23*)* 4
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        let path = vec![1, 2, 3, 3, 3, 2, 3, 3, 3, 2, 3, 3, 3, 3, 4];
        pfp.reduce_fun1(&mut &path[..], 2);
    }
    #[test]
    fn test8() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23*)* 4
        pfp.first_to_lasts.insert(1, [5].into_iter().collect());
        let path = vec![1, 3, 2, 2, 3, 4, 3, 5];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test8_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // f = 1 (23*)* 4
        pfp.first_to_lasts.insert(1, [5].into_iter().collect());
        let path = vec![1, 3, 2, 2, 3, 4, 3, 5];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test9() {
        let mut pfp = PerFunctionPathCoverage::empty();
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        pfp.first_to_lasts.insert(2, [3].into_iter().collect());
        let path = vec![1, 2, 3, 2, 3, 4];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test9_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        pfp.first_to_lasts.insert(1, [4].into_iter().collect());
        pfp.first_to_lasts.insert(2, [3].into_iter().collect());
        let path = vec![1, 2, 3, 2, 3, 4];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test_nested_loops() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // Function with nested loops:
        // 1 -> (2 -> (3 -> 4)* -> 5)* -> 6
        pfp.first_to_lasts.insert(1, [6].into_iter().collect());
        // Path that exercises nested loops multiple times:
        // Outer loop iterates twice
        // Inner loop iterates 2-3 times per outer iteration
        let path = vec![
            1,                  // Enter function
                2,             // Enter outer loop first time
                    3,        // Enter inner loop
                    4,        // Inner loop iteration 1
                    3,        // Enter inner loop again  
                    4,        // Inner loop iteration 2
                5,            // Exit inner loop
                2,             // Enter outer loop second time
                    3,        // Enter inner loop
                    4,        // Inner loop iteration 1
                    3,        // Enter inner loop again
                    4,        // Inner loop iteration 2
                    3,        // Enter inner loop again
                    4,        // Inner loop iteration 3
                5,            // Exit inner loop
            6                   // Exit function
        ];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test_nested_loops_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        pfp.first_to_lasts.insert(1, [6].into_iter().collect());
        let path = vec![
            1,                  // Enter function
                2,             // Enter outer loop first time
                    3,        // Enter inner loop
                    4,        // Inner loop iteration 1
                    3,        // Enter inner loop again  
                    4,        // Inner loop iteration 2
                5,            // Exit inner loop
                2,             // Enter outer loop second time
                    3,        // Enter inner loop
                    4,        // Inner loop iteration 1
                    3,        // Enter inner loop again
                    4,        // Inner loop iteration 2
                    3,        // Enter inner loop again
                    4,        // Inner loop iteration 3
                5,            // Exit inner loop
            6                   // Exit function
        ];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test_deep_nested_loops() {
        let mut pfp = PerFunctionPathCoverage::empty();
        // Function with triple-nested loops:
        // 1 -> (2 -> (3 -> (4 -> 5)* -> 6)* -> 7)* -> 8
        pfp.first_to_lasts.insert(1, [8].into_iter().collect());
        // Path that exercises nested loops multiple times:
        // Outermost loop iterates three times
        // Middle loop iterates 2-3 times per outer iteration
        // Innermost loop iterates 1-3 times per middle iteration
        let path = vec![
            1,                            // Enter function
                2,                        // Enter outermost loop first time
                    3,                    // Enter middle loop first time
                        4, 5,            // Innermost loop once
                    6,                   // Exit innermost loop
                    3,                   // Enter middle loop second time
                        4, 5, 4, 5, 4, 5,  // Innermost loop three times
                    6,                   // Exit middle loop
                7,                       // Exit outermost iteration
                2,                       // Enter outermost loop second time
                    3,                   // Enter middle loop
                        4, 5, 4, 5,      // Innermost loop twice
                    6,                   // Exit innermost
                    3,                   // Middle loop again
                        4, 5,            // Innermost once more
                    6,                   // Exit all loops
                7,                       // Exit second iteration
                2,                       // Enter outermost loop third time
                    3,                   // Enter middle loop
                        4, 5,     // Innermost loop twice
                    6,                   // Exit innermost
                    3,                   // Middle loop again
                        4, 5, 4, 5, 4, 5,  // Innermost once more
                    6,                   // Exit all loops
                7,                       // Exit third iteration
            8                            // Exit function
        ];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test_deep_nested_loops_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        pfp.first_to_lasts.insert(1, [8].into_iter().collect());
        let path = vec![
            1,                            
                2,                        
                    3,                    
                        4, 5,            
                    6,                   
                    3,                   
                        4, 5, 4, 5, 4, 5,      
                    6,                   
                7,                       
                2,                       
                    3,                   
                        4, 5, 4, 5,      
                    6,                   
                    3,                   
                        4, 5,            
                    6,                   
                7,
                2,                       
                    3,                   
                        4, 5,   
                    6,                   
                    3,                   
                        4, 5, 4, 5, 4, 5,      
                    6,                   
                7,
            8                            
        ];
        pfp.reduce_fun1(&mut &path[..], 2);
    }

    #[test]
    fn test_single_loop() {
        let mut pfp = PerFunctionPathCoverage::empty();
        pfp.first_to_lasts.insert(1, [3].into_iter().collect());
        let path = vec![1, 2, 2, 2, 2, 2, 2, 2, 3];
        pfp.reduce_fun(&mut &path[..]);
    }

    #[test]
    fn test_single_loop_() {
        let mut pfp = PerFunctionPathCoverage::empty();
        pfp.first_to_lasts.insert(1, [3].into_iter().collect());
        let path = vec![1, 2, 2, 2, 2, 2, 2, 2, 3];
        pfp.reduce_fun1(&mut &path[..], 2);
    }
}
