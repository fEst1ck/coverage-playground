use super::{CoverageFeedback, CoverageMetric};
use md5::{compute, Digest};
use rustc_hash::FxHashSet;
use serde_json::Value;

pub struct RawPathCoverage {
    paths: FxHashSet<Digest>,
}

impl Default for RawPathCoverage {
    fn default() -> Self {
        Self {
            paths: FxHashSet::default(),
        }
    }
}

impl CoverageMetric for RawPathCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback {
        // Convert Vec<u32> to bytes
        let bytes: Vec<u8> = path
            .iter()
            .flat_map(|&num| num.to_ne_bytes())
            .collect();

        // Compute hash of the byte representation
        let path_hash = compute(&bytes);

        // Return true if this is a new path
        let new_path = self.paths.insert(path_hash);
        if new_path {
            CoverageFeedback::NewPath { block_uniqueness: 0, edge_uniqueness: 0 }
        } else {
            CoverageFeedback::NoCoverage
        }
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.paths.len().into())
    }

    fn name(&self) -> &'static str {
        "rawpath"
    }

    fn priority(&self) -> usize {
        10
    }
}
