use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Write,
};

use itertools::Itertools;
use path_reduction::json_parser::parse_json_file;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::{json, Value};

/// Control flow graph information of the target program
struct ControlFlowGraphInfo {
    fun_id_to_name: FxHashMap<u32, String>,
    block_id_to_fun_id: FxHashMap<u32, u32>,
    fun_id_to_all_blocks: FxHashMap<u32, BTreeSet<u32>>,
}

impl ControlFlowGraphInfo {
    /// Create a new ControlFlowGraphInfo from a file of JSON objects
    pub fn from_json(path: &str) -> Self {
        let mut fun_id_to_name = FxHashMap::default();
        let mut block_id_to_fun_id = FxHashMap::default();
        let mut fun_id_to_all_blocks = FxHashMap::default();
        let modules = parse_json_file(path).unwrap();
        for module in modules {
            for func in module.functions {
                fun_id_to_name.insert(
                    func.entry_block,
                    format!("{}.{}", module.module_name, func.name),
                );
                for block in func.all_blocks {
                    block_id_to_fun_id.insert(block, func.entry_block);
                    fun_id_to_all_blocks.entry(func.entry_block).or_insert(BTreeSet::new()).insert(block);
                }
            }
        }
        Self {
            fun_id_to_name,
            block_id_to_fun_id,
            fun_id_to_all_blocks,
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
    pub fn analyze_fun_coverage(
        &self,
        block_coverage: &Value,
        edge_coverage: &Value,
    ) -> FunctionCoverage {
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
                .or_insert(EachFunctionCoverage::new(fun_id, fun_name));

            // update unique blocks covered
            *entry.unique_blocks.entry(block_id).or_default() += count;
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
            let _count = elem[2].as_u64().unwrap() as usize;
            let fun_id = self.control_flow_graph_info.block_id_to_fun_id[&src];
            let fun_name = self.control_flow_graph_info.fun_id_to_name[&fun_id].clone();
            let entry = fun_coverage
                .coverage
                .entry(fun_id)
                .or_insert(EachFunctionCoverage::new(fun_id, fun_name));
            // update unique edges covered
            entry.unique_edges.insert((src, dst));
            // update callees
            if self.control_flow_graph_info.fun_id_to_name.contains_key(&dst) {
                entry.calls.push(dst);
            }
        }
        fun_coverage.max_exec_per_fun = fun_coverage
            .coverage
            .values()
            .map(|coverage| coverage.nums_executed)
            .max()
            .unwrap_or(0);

        for (fun_id, coverage) in &mut fun_coverage.coverage {
            let all_blocks = &self.control_flow_graph_info.fun_id_to_all_blocks[fun_id];
            for block in all_blocks {
                coverage.unique_blocks.entry(*block).or_insert(0);
            }
        }

        // hyper edges
        let hyper_edges = self.analyze_hyper_edge(block_coverage, edge_coverage);
        for (pred, succs) in &hyper_edges {
            let fun_id = self.control_flow_graph_info.block_id_to_fun_id[&pred];
            fun_coverage.coverage.get_mut(&fun_id).unwrap().hyper_edges.extend(succs.iter().map(|succ| (*pred, *succ)));
        }

        fun_coverage
    }

    /// Analyze the function coverage information
    /// block_coverage: the coverage information of the blocks, an array of [block_id, count]
    /// edge_coverage: the coverage information of the edges, an array of [src, dst, count]
    pub fn analyze_hyper_edge(
        &self,
        _block_coverage: &Value,
        edge_coverage: &Value,
    ) -> BTreeMap<u32, BTreeSet<u32>> {
        let mut succs = FxHashMap::default();
        // Add edges with edge counts
        for elem in edge_coverage.as_array().unwrap() {
            let src = elem[0].as_u64().unwrap() as u32;
            let dst = elem[1].as_u64().unwrap() as u32;
            succs.entry(src).or_insert(BTreeSet::new()).insert(dst);
        }
        let mut hyper_edges = BTreeMap::new();
        // for each node n, find all paths starting from n, and add the first node
        // in the function of n to the hyper edge
        for block in succs.keys() {
            let cur_fun = self.control_flow_graph_info.block_id_to_fun_id[block];
            let mut explored = FxHashSet::default();
            let mut stack = succs[block].iter().cloned().collect_vec();
            while let Some(suc) = stack.pop() {
                if explored.insert(suc) {
                    let suc_fun = self.control_flow_graph_info.block_id_to_fun_id[&suc];
                    if suc_fun == cur_fun {
                        hyper_edges.entry(*block).or_insert(BTreeSet::new()).insert(suc);
                    } else {
                        let suc_suc = succs.get(&suc).cloned().unwrap_or_default();
                        stack.extend(suc_suc.into_iter());
                    }
                }
            }
        }
        hyper_edges
    }

    pub fn write_fun_coverage(
        &self,
        fun_coverage: &FunctionCoverage,
        path: &str,
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(
            fun_coverage
                .generate_dot(&self.control_flow_graph_info)
                .as_bytes(),
        )?;
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
struct EachFunctionCoverage {
    /// Function id
    id: u32,
    /// Function name
    name: String,
    /// Number of times the function was executed
    nums_executed: usize,
    /// unique blocks covered
    /// Maps block id to the number of times it was executed
    unique_blocks: BTreeMap<u32, usize>,
    /// unique edges covered
    unique_edges: BTreeSet<(u32, u32)>,
    /// other functions that are called by this function
    calls: Vec<u32>,
    /// hyper edges
    hyper_edges: BTreeSet<(u32, u32)>,
}

impl EachFunctionCoverage {
    fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            nums_executed: 0,
            unique_blocks: BTreeMap::new(),
            unique_edges: BTreeSet::new(),
            calls: Vec::new(),
            hyper_edges: BTreeSet::new(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name.clone(),
            "nums_executed": self.nums_executed,
            "unique_blocks": self.unique_blocks.iter().map(|(block, count)| json!([*block as u64, *count as u64])).collect_vec(),
            "unique_edges": self.unique_edges.iter().map(|&(src, dst)| json!([src, dst])).collect_vec(),
            "calls": self.calls.clone(),
            "hyper_edges": self.hyper_edges.iter().map(|&(src, dst)| json!([src, dst])).collect_vec(),
        })
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

    pub fn to_json(&self) -> Value {
        let mut vec = Vec::new();
        for (_fun_id, coverage) in &self.coverage {
            vec.push(coverage.to_json());
        }
        Value::Array(vec)
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
                let intensity = (num_exec_per_fun as f32 / self.max_exec_per_fun as f32) as f32;
                // Convert intensity to RGB values: green (0,1,0) to red (1,0,0)
                let r = (intensity * 255.0) as u8;
                let g = ((1.0 - intensity) * 255.0) as u8;
                let b = 0u8;
                format!("#{:02x}{:02x}{:02x}", r, g, b)
            };

            let label = format!(
                "{}\nExecutions: {}\nBlocks: {}\nEdges: {}",
                coverage.name,
                coverage.nums_executed,
                coverage.unique_blocks.len(),
                coverage.unique_edges.len(),
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
    pub fn write_dot_file(
        &self,
        path: &str,
        control_flow_graph_info: &ControlFlowGraphInfo,
    ) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;
        let dot = self.generate_dot(control_flow_graph_info);
        let mut file = File::create(path)?;
        file.write_all(dot.as_bytes())?;
        Ok(())
    }
}
