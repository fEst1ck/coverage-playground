use super::CoverageMetric;
use rustc_hash::FxHashSet;
use serde_json::Value;

#[derive(Default)]
pub struct EdgeCoverage {
    edges: FxHashSet<(u32, u32)>,
}

impl CoverageMetric for EdgeCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> bool {
        let mut new_coverage = false;

        for window in path.windows(2) {
            let edge = (window[0], window[1]);
            new_coverage |= self.edges.insert(edge);
        }

        new_coverage
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.edges.len().into())
    }

    fn full_cov(&self) -> Value {
        Value::Array(self.edges.iter().map(|e| Value::Array(vec![Value::Number(e.0.into()), Value::Number(e.1.into())])).collect())
    }

    fn name(&self) -> &'static str {
        "edge"
    }

    fn priority(&self) -> usize {
        90
    }
}
