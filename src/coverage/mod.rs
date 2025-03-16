//！ This module implements the coverage metrics
// Stuff to look at:
// `CoverageMetric` trait, and its implementations
// `CoverageMetricAggregator` struct

mod block;
mod edge;
mod path;
use std::collections::BTreeMap;

pub use block::BlockCoverage;
pub use edge::EdgeCoverage;
pub use path::PathCoverage;
use serde_json::Value;

pub trait CoverageMetric {
    /// Update coverage with the given path and return true if new coverage was found
    fn update_from_path(&mut self, path: &[u32]) -> bool;

    /// Get the coverage information
    fn cov_info(&self) -> Value;

    fn full_cov(&self) -> Value {
        Value::Null
    }

    /// Get the name of the metric
    fn name(&self) -> &'static str {
        ""
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

pub type CoverageFeedback<'a> = BTreeMap<&'a str, bool>;

/// Track multiple coverage metrics simultaneously
#[derive(Default)]
pub struct CoverageMetricAggregator {
    metrics: Vec<Box<dyn CoverageMetric>>,
}

impl CoverageMetricAggregator {
    pub fn new(metrics: Vec<Box<dyn CoverageMetric>>) -> Self {
        Self { metrics }
    }

    pub fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback<'static> {
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
