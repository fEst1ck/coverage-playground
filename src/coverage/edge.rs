use super::CoverageMetric;
use rustc_hash::FxHashSet;

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
}
