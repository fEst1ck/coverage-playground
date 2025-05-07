use std::{fs::File, io::Write};

use path_reduction::json_parser::parse_json_file;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

/// Control flow graph information of the target program
struct ControlFlowGraphInfo {
    fun_id_to_name: FxHashMap<u32, String>,
    block_id_to_fun_id: FxHashMap<u32, u32>,
}

impl ControlFlowGraphInfo {
    /// Create a new ControlFlowGraphInfo from a file of JSON objects
    pub fn from_json(path: &str) -> Self {
        let mut fun_id_to_name = FxHashMap::default();
        let mut block_id_to_fun_id = FxHashMap::default();
        let modules = parse_json_file(path).unwrap();
        for module in modules {
            for func in module.functions {
                fun_id_to_name.insert(
                    func.entry_block,
                    format!("{}.{}", module.module_name, func.name),
                );
                for block in func.all_blocks {
                    block_id_to_fun_id.insert(block, func.entry_block);
                }
            }
        }
        Self {
            fun_id_to_name,
            block_id_to_fun_id,
        }
    }
}

impl Default for ControlFlowGraphInfo {
    fn default() -> Self {
        let cfg_file = std::env::var("CFG_FILE").unwrap_or_default();
        Self::from_json(&cfg_file)
    }
}

/// Analyzer for the coverage information
pub struct Analyzer {
    control_flow_graph_info: ControlFlowGraphInfo,
}

impl Analyzer {
    /// Analyze the function coverage information
    /// block_coverage: the coverage information of the blocks, an array of [block_id, count]
    /// edge_coverage: the coverage information of the edges, an array of [src, dst, count]
    pub fn analyze_fun_coverage(&self, block_coverage: &Value, edge_coverage: &Value) -> FunctionCoverage {
        let mut fun_coverage = FunctionCoverage::new();
        // Add nodes with block counts
        for elem in block_coverage.as_array().unwrap() {
            let block_id = elem[0].as_u64().unwrap() as u32;
            let count = elem[1].as_u64().unwrap() as usize;
            let fun_id = self.control_flow_graph_info.block_id_to_fun_id[&block_id];
            let fun_name = self.control_flow_graph_info.fun_id_to_name[&fun_id].clone();
            let entry = fun_coverage
                .coverage
                .entry(fun_id)
                .or_insert(EachFunctionCoverage::new(fun_name));

            // update cummulative block coverage
            entry.cummulative_block_cov += count;
            // update unique blocks covered
            entry.unique_blocks.insert(block_id);
            // update number of times the function was executed
            // which is the number of times the first block of the function was executed
            if let Some(name) = self.control_flow_graph_info.fun_id_to_name.get(&block_id) {
                entry.nums_executed += count;
                entry.name = name.clone();
            }
        }

        // Add edges with edge counts
        for elem in edge_coverage.as_array().unwrap() {
            let src = elem[0].as_u64().unwrap() as u32;
            let dst = elem[1].as_u64().unwrap() as u32;
            let count = elem[2].as_u64().unwrap() as usize;
            let fun_id = self.control_flow_graph_info.block_id_to_fun_id[&src];
            let fun_name = self.control_flow_graph_info.fun_id_to_name[&fun_id].clone();
            let entry = fun_coverage
                .coverage
                .entry(fun_id)
                .or_insert(EachFunctionCoverage::new(fun_name));
            // update cummulative edge coverage
            entry.cummulative_edge_cov += count;
            // update unique edges covered
            entry.unique_edges.insert((src, dst));
        }
        fun_coverage.max_exec_per_fun = fun_coverage
            .coverage
            .values()
            .map(|coverage| coverage.nums_executed)
            .max()
            .unwrap_or(0);
        fun_coverage
    }

    pub fn write_fun_coverage(&self, fun_coverage: &FunctionCoverage, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(fun_coverage.generate_dot(&self.control_flow_graph_info).as_bytes())?;
        Ok(())
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self {
            control_flow_graph_info: ControlFlowGraphInfo::default(),
        }
    }
}

/// Coverage information of each function
#[derive(Default)]
struct EachFunctionCoverage {
    /// Function name
    name: String,
    /// Number of times the function was executed
    nums_executed: usize,
    /// unique blocks covered
    unique_blocks: FxHashSet<u32>,
    /// Number of cummulative blocks covered
    cummulative_block_cov: usize,
    /// unique edges covered
    unique_edges: FxHashSet<(u32, u32)>,
    /// Number of cummulative edges covered
    cummulative_edge_cov: usize,
}

impl EachFunctionCoverage {
    fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

pub struct FunctionCoverage {
    /// Function Coverage of each function
    coverage: FxHashMap<u32, EachFunctionCoverage>,
    max_exec_per_fun: usize,
}

impl FunctionCoverage {
    fn new() -> Self {
        Self {
            coverage: FxHashMap::default(),
            max_exec_per_fun: 0,
        }
    }

    /// Generate a Graphviz DOT visualization of the function coverage
    pub fn generate_dot(&self, control_flow_graph_info: &ControlFlowGraphInfo) -> String {
        let mut dot = String::from("digraph function_coverage {\n");
        dot.push_str("    node [shape=box, style=filled];\n");
        dot.push_str("    rankdir=LR;\n\n");

        // Add nodes for each function
        for (fun_id, coverage) in &self.coverage {
            let num_exec_per_fun = coverage.nums_executed;
            let color = {
                // Color gradient from light green to dark red based on relative execution count
                let intensity = 
                    (num_exec_per_fun as f32 / self.max_exec_per_fun as f32 * 0.7) as f32;
                // Convert intensity to RGB values: green (0,1,0) to red (1,0,0)
                let r = (intensity * 255.0) as u8;
                let g = ((1.0 - intensity) * 255.0) as u8;
                let b = 0u8;
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            };

            let label = format!(
                "{}\nExecutions: {}\nBlocks: {}/{}\nEdges: {}/{}",
                coverage.name,
                coverage.nums_executed,
                coverage.unique_blocks.len(),
                coverage.cummulative_block_cov,
                coverage.unique_edges.len(),
                coverage.cummulative_edge_cov
            );

            dot.push_str(&format!(
                "    {} [label=\"{}\", fillcolor=\"{}\"];\n",
                fun_id, label, color
            ));
        }

        // Add edges between functions
        for (fun_id, coverage) in &self.coverage {
            for (_src, dst) in &coverage.unique_edges {
                if control_flow_graph_info.fun_id_to_name.contains_key(&dst) {
                    dot.push_str(&format!("    {} -> {};\n", fun_id, dst));
                }
            }
        }

        dot.push_str("}\n");
        dot
    }

    /// Write the function coverage visualization to a DOT file
    pub fn write_dot_file(&self, path: &str, control_flow_graph_info: &ControlFlowGraphInfo) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;
        let dot = self.generate_dot(control_flow_graph_info);
        let mut file = File::create(path)?;
        file.write_all(dot.as_bytes())?;
        Ok(())
    }
}
