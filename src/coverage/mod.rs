mod all;
mod block;
mod edge;
mod path;
use std::collections::BTreeMap;

use all::AllCoverage;
pub use block::BlockCoverage;
pub use edge::EdgeCoverage;
pub use path::PathCoverage;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum CoverageType {
    Block,
    Edge,
    Path,
}

impl Default for CoverageType {
    fn default() -> Self {
        CoverageType::Edge
    }
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
    /// Update coverage with the given path and return true if new coverage was found
    fn update_from_path(&mut self, path: &[u32]) -> bool;

    /// Get the coverage information
    fn cov_info(&self) -> Value;

    /// Get the name of the metric
    fn name(&self) -> &str {
        ""
    }
}

pub fn create_coverage_metric(coverage_type: CoverageType, all_coverage: bool) -> Box<dyn CoverageMetric> {
    if all_coverage {
        Box::new(AllCoverage::new(coverage_type))
    } else {
        match coverage_type {
            CoverageType::Block => Box::new(BlockCoverage::default()),
            CoverageType::Edge => Box::new(EdgeCoverage::default()),
            CoverageType::Path => Box::new(PathCoverage::default()),
        }
    }
}

/// Get a coverage metric by name
pub fn get_coverage_metric_by_name(name: &str) -> Option<Box<dyn CoverageMetric>> {
    match name {
        "block" => Some(Box::new(BlockCoverage::default())),
        "edge" => Some(Box::new(EdgeCoverage::default())),
        "path" => Some(Box::new(PathCoverage::default())),
        _ => None,
    }
}

/// Track multiple coverage metrics simultaneously
#[derive(Default)]
pub struct CoverageMetricAggregator {
    metrics: Vec<Box<dyn CoverageMetric>>,
}

impl CoverageMetricAggregator {
    pub fn new(metrics: Vec<Box<dyn CoverageMetric>>) -> Self {
        Self { metrics }
    }

    pub fn update_from_path(&mut self, path: &[u32]) -> BTreeMap<&str, bool> {
        let mut results = BTreeMap::new();

        for metric in &mut self.metrics {
            let updated = metric.update_from_path(path);
            results.insert(metric.name(), updated);
        }

        results
    }

    pub fn cov_info(&self) -> Value {
        let mut info = serde_json::Map::new();
        for metric in &self.metrics {
            info.insert(metric.name().to_string(), metric.cov_info());
        }
        Value::Object(info)
    }
}
