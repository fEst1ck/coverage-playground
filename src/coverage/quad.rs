use crate::coverage::EdgeCoverage;

use super::{CoverageFeedback, CoverageMetric};
use rustc_hash::FxHashMap;
use serde_json::Value;

#[derive(Default)]
pub struct QuadCoverage {
    edges: FxHashMap<(u32, u32), usize>,
    raw_edge: EdgeCoverage,
}

impl CoverageMetric for QuadCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback {
        let raw_edge_feedback = self.raw_edge.update_from_path(path);

        let mut new_coverage = false;

        let mut uniq = usize::MAX;

        for (i, block) in path.iter().enumerate() {
            let j = (i + path.len() - 1) / 2;
            let succ = path[j];
            let edge = (*block, succ);
            let count = *self.edges.entry(edge).and_modify(|count| *count += 1).or_insert_with(|| {
            new_coverage = true;
            1
            });
            uniq = uniq.min(count);
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
