use crate::coverage::EdgeCoverage;

use super::{CoverageFeedback, CoverageMetric};
use rustc_hash::{FxHashMap, FxHashSet};
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
        let mut prev_blocks: FxHashSet<u32> = FxHashSet::default();

        for block in path {
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
        "quad"
    }

    fn priority(&self) -> usize {
        90
    }
}
