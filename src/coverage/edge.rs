use super::CoverageMetric;
use rustc_hash::FxHashMap;
use serde_json::Value;

#[derive(Default)]
pub struct EdgeCoverage {
    edges: FxHashMap<(u32, u32), usize>,
}

impl CoverageMetric for EdgeCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> bool {
        let mut new_coverage = false;

        for window in path.windows(2) {
            let edge = (window[0], window[1]);
            self.edges.entry(edge).and_modify(|count| *count += 1).or_insert_with(|| {
                new_coverage = true;
                1
            });
        }

        new_coverage
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.edges.len().into())
    }

    // an array of [from, to, count]
    fn full_cov(&self) -> Value {
        Value::Array(
            self.edges
                .iter()
                .map(|(edge, count)| {
                    Value::Array(vec![
                        Value::Number((edge.0).into()),
                        Value::Number((edge.1).into()),
                        Value::Number((*count).into()),
                    ])
                })
                .collect(),
        )
    }

    fn name(&self) -> &'static str {
        "edge"
    }

    fn priority(&self) -> usize {
        90
    }
}
