use super::{CoverageFeedback, CoverageMetric};
use rustc_hash::FxHashMap;
use serde_json::Value;

#[derive(Default)]
pub struct BlockCoverage {
    blocks: FxHashMap<u32, usize>,
}

impl CoverageMetric for BlockCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback {
        let mut new_coverage = false;

        let mut uniq = usize::MAX;

        for &block in path {
            let count = *self.blocks
                .entry(block)
                .and_modify(|count| *count += 1)
                .or_insert_with(|| {
                    new_coverage = true;
                    1
                });
            uniq = uniq.min(count);
        }
        
        if new_coverage {
            CoverageFeedback::NewBlock { uniqueness: uniq }
        } else {
            CoverageFeedback::NoCoverage(uniq)
        }
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.blocks.len().into())
    }

    // an array of [block, count]
    fn full_cov(&self) -> Value {
        Value::Array(
            self.blocks
                .iter()
                .map(|(block, count)| {
                    Value::Array(vec![
                        Value::Number((*block).into()),
                        Value::Number((*count).into()),
                    ])
                })
                .collect(),
        )
    }

    fn name(&self) -> &'static str {
        "block"
    }

    fn priority(&self) -> usize {
        100
    }
}
