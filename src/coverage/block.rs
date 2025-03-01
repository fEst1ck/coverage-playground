use super::CoverageMetric;
use rustc_hash::FxHashSet;

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
}
