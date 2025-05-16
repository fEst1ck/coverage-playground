use super::{CoverageFeedback, CoverageMetric};
use crate::coverage::block::BlockCoverage;
use crate::coverage::edge::EdgeCoverage;
use md5::{compute, Digest};
use path_reduction::path_reduction::PathReducer;
use rustc_hash::FxHashSet;
use serde_json::Value;

pub struct PathCoverage {
    block_cov: BlockCoverage,
    edge_cov: EdgeCoverage,
    paths: FxHashSet<Digest>,
    path_reduction: PathReducer<u32, u32>,
}

impl Default for PathCoverage {
    fn default() -> Self {
        Self {
            block_cov: BlockCoverage::default(),
            edge_cov: EdgeCoverage::default(),
            paths: FxHashSet::default(),
            path_reduction: {
                let cfg_file = std::env::var("CFG_FILE").unwrap_or_default();
                PathReducer::from_json(&cfg_file)
            },
        }
    }
}

impl CoverageMetric for PathCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback {
        let reduced_path = self.path_reduction.simple_reduce(path);

        if std::env::var("DEBUG").unwrap_or_default() == "1" {
            eprintln!(
                "Path len: {:?}\nreduced path len: {:?}",
                path.len(),
                reduced_path.len()
            );
        }

        // Convert Vec<u32> to bytes
        let bytes: Vec<u8> = reduced_path
            .iter()
            .flat_map(|&num| num.to_ne_bytes())
            .collect();

        // Compute hash of the byte representation
        let path_hash = compute(&bytes);

        // Return true if this is a new path
        let new_path = self.paths.insert(path_hash);

        let block_feedback = self.block_cov.update_from_path(&reduced_path);
        let edge_feedback = self.edge_cov.update_from_path(&reduced_path);
        if block_feedback.new_cov() {
            block_feedback
        } else if edge_feedback.new_cov() {
            edge_feedback
        } else if new_path {
            CoverageFeedback::NewPath {
                block_uniqueness: block_feedback.get_block_uniqueness(),
                edge_uniqueness: edge_feedback.get_edge_uniqueness(),
            }
        } else {
            CoverageFeedback::NoCoverage
        }
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.paths.len().into())
    }

    fn name(&self) -> &'static str {
        "path"
    }

    fn priority(&self) -> usize {
        10
    }
}
