use crate::coverage::{BlockCoverage, CoverageMetric, EdgeCoverage, PathCoverage};

pub struct AllCoverage {
    block_coverage: BlockCoverage,
    edge_coverage: EdgeCoverage,
    path_coverage: PathCoverage,
}

impl AllCoverage {}

impl CoverageMetric for AllCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> bool {
        self.block_coverage.update_from_path(path)
            || self.edge_coverage.update_from_path(path)
            || self.path_coverage.update_from_path(path)
    }
}
