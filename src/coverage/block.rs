use super::CoverageMetric;
use rustc_hash::FxHashSet;
use serde_json::Value;

#[derive(Default)]
pub struct BlockCoverage {
    blocks: FxHashSet<u32>,
}

impl CoverageMetric for BlockCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> bool {
        let mut new_coverage = false;

        for block in path {
            new_coverage |= self.blocks.insert(*block);
        }

        new_coverage
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.blocks.len().into())
    }

    fn full_cov(&self) -> Value {
        Value::Array(
            self.blocks
                .iter()
                .map(|b| Value::Number((*b).into()))
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
