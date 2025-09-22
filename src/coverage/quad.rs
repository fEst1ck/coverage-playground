use crate::coverage::EdgeCoverage;

use super::{CoverageFeedback, CoverageMetric};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

use path_reduction::json_parser::parse_json_file;

type BlockID = u32;

pub struct QuadCoverage {
    edges: FxHashMap<(u32, u32), usize>,
    raw_edge: EdgeCoverage,
    first_to_lasts: FxHashMap<BlockID, FxHashSet<BlockID>>,
}

impl QuadCoverage {
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
            edges: Default::default(),
            raw_edge: EdgeCoverage::default(),
            first_to_lasts,
        }
    }
}

impl Default for QuadCoverage {
    fn default() -> Self {
        let cfg_file = std::env::var("CFG_FILE").unwrap_or_default();
        Self::from_json(&cfg_file)
    }
}

impl CoverageMetric for QuadCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback {
        let raw_edge_feedback = self.raw_edge.update_from_path(path);

        let mut new_coverage = false;

        let mut uniq = usize::MAX;
        let mut prev_blocks: FxHashSet<u32> = FxHashSet::default();

        for block in path {
            if !self.first_to_lasts.contains_key(block) {
                continue;
            }
            for prev_block in &prev_blocks {
                let edge = (*prev_block, *block);
                let count = *self.edges.entry(edge).and_modify(|count| *count += 1).or_insert_with(|| {
                    new_coverage = true;
                    1
                });
                uniq = uniq.min(count);
            }
            prev_blocks.insert(*block);
        }

        if raw_edge_feedback.new_cov() {
            raw_edge_feedback
        } else if new_coverage {
            CoverageFeedback::NewQuad{ uniqueness: uniq }
        } else {
            CoverageFeedback::NoCoverage(raw_edge_feedback.get_edge_uniqueness())
        }
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.edges.len().into())
    }

    fn name(&self) -> &'static str {
        "quad"
    }

    fn priority(&self) -> usize {
        90
    }
}
