mod block;
mod edge;
mod path;

pub use block::BlockCoverage;
pub use edge::EdgeCoverage;
pub use path::PathCoverage;

#[derive(Debug, Clone, Copy)]
pub enum CoverageType {
    Block,
    Edge,
    Path,
}

impl std::str::FromStr for CoverageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "block" => Ok(CoverageType::Block),
            "edge" => Ok(CoverageType::Edge),
            "path" => Ok(CoverageType::Path),
            _ => Err(format!("Unknown coverage type: {}", s)),
        }
    }
}

pub trait CoverageMetric {
    // Update coverage with the given path and return true if new coverage was found
    fn update_from_path(&mut self, path: &[u32]) -> bool;
}

pub fn create_coverage_metric(coverage_type: CoverageType) -> Box<dyn CoverageMetric> {
    match coverage_type {
        CoverageType::Block => Box::new(BlockCoverage::default()),
        CoverageType::Edge => Box::new(EdgeCoverage::default()),
        CoverageType::Path => Box::new(PathCoverage::default()),
    }
} 