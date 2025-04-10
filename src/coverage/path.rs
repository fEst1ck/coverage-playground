use super::CoverageMetric;
use md5::{compute, Digest};
use path_reduction::path_reduction::PathReducer;
use rustc_hash::FxHashSet;
use serde_json::Value;

pub struct PathCoverage {
    paths: FxHashSet<Digest>,
    path_reduction: PathReducer<u32, u32>,
}

impl Default for PathCoverage {
    fn default() -> Self {
        Self {
            paths: FxHashSet::default(),
            path_reduction: {
                let cfg_file = std::env::var("CFG_FILE").unwrap_or_default();
                PathReducer::from_json(&cfg_file)
            },
        }
    }
}

impl CoverageMetric for PathCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> bool {
        let reduced_path = self.path_reduction.simple_reduce(path);

        if std::env::var("DEBUG").unwrap_or_default() == "1" {
            eprintln!("Path len: {:?}\nreduced path len: {:?}", path.len(), reduced_path.len());
        }

        // Convert Vec<u32> to bytes
        let bytes: Vec<u8> = reduced_path
            .iter()
            .flat_map(|&num| num.to_ne_bytes())
            .collect();

        // Compute hash of the byte representation
        let path_hash = compute(&bytes);

        // Return true if this is a new path
        self.paths.insert(path_hash)
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
