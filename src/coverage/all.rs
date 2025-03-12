use crate::coverage::{BlockCoverage, CoverageMetric, CoverageType, EdgeCoverage, PathCoverage};
use serde_json::{self, Value};

#[derive(Default)]
pub struct AllCoverage {
    block_coverage: BlockCoverage,
    edge_coverage: EdgeCoverage,
    path_coverage: PathCoverage,
    coverage_type: CoverageType,
}

impl AllCoverage {
    pub fn new(coverage_type: CoverageType) -> Self {
        Self {
            block_coverage: BlockCoverage::default(),
            edge_coverage: EdgeCoverage::default(),
            path_coverage: PathCoverage::default(),
            coverage_type,
        }
    }
}

impl CoverageMetric for AllCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> bool {
        let new_block = self.block_coverage.update_from_path(path);
        let new_edge = self.edge_coverage.update_from_path(path);
        let new_path = self.path_coverage.update_from_path(path);

        match self.coverage_type {
            CoverageType::Block => new_block,
            CoverageType::Edge => new_edge,
            CoverageType::Path => new_path,
        }
    }

    fn cov_info(&self) -> Value {
        serde_json::json!({
            "block": self.block_coverage.cov_info(),
            "edge": self.edge_coverage.cov_info(),
            "path": self.path_coverage.cov_info(),
        })
    }
}
