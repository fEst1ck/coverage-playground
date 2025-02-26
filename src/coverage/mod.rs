use rustc_hash::FxHashSet;

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
    fn update_from_path(&mut self, path: &[u32]);
    fn has_new_coverage(&self, path: &[u32]) -> bool;
}

#[derive(Default)]
pub struct BlockCoverage {
    blocks: FxHashSet<u32>,
}

impl CoverageMetric for BlockCoverage {
    fn update_from_path(&mut self, path: &[u32]) {
        self.blocks.extend(path.iter().copied());
    }

    fn has_new_coverage(&self, path: &[u32]) -> bool {
        path.iter().any(|block| !self.blocks.contains(block))
    }
}

#[derive(Default)]
pub struct EdgeCoverage {
    edges: FxHashSet<(u32, u32)>,
}

impl CoverageMetric for EdgeCoverage {
    fn update_from_path(&mut self, path: &[u32]) {
        for window in path.windows(2) {
            self.edges.insert((window[0], window[1]));
        }
    }

    fn has_new_coverage(&self, path: &[u32]) -> bool {
        path.windows(2)
            .any(|window| !self.edges.contains(&(window[0], window[1])))
    }
}

#[derive(Default)]
pub struct PathCoverage {
    paths: FxHashSet<Vec<u32>>,
}

impl CoverageMetric for PathCoverage {
    fn update_from_path(&mut self, path: &[u32]) {
        self.paths.insert(path.to_vec());
    }

    fn has_new_coverage(&self, path: &[u32]) -> bool {
        !self.paths.contains(path)
    }
}

pub fn create_coverage_metric(coverage_type: CoverageType) -> Box<dyn CoverageMetric> {
    match coverage_type {
        CoverageType::Block => Box::new(BlockCoverage::default()),
        CoverageType::Edge => Box::new(EdgeCoverage::default()),
        CoverageType::Path => Box::new(PathCoverage::default()),
    }
} 