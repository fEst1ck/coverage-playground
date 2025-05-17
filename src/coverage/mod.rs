//！ This module implements the coverage metrics
// Stuff to look at:
// `CoverageMetric` trait, and its implementations
// `CoverageMetricAggregator` struct

mod block;
mod edge;
mod path;
mod per_function;
mod raw_path;
use std::{any::Any, cmp::Ordering, collections::BTreeMap};

pub use block::BlockCoverage;
use cached::proc_macro::cached;
pub use edge::EdgeCoverage;
pub use path::PathCoverage;
use per_function::PerFunctionPathCoverage;
use raw_path::RawPathCoverage;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverageFeedback {
    NewBlock {
        uniqueness: usize,
    },
    NewEdge {
        uniqueness: usize,
    },
    NewPath {
        uniqueness: usize,
    },
    NoCoverage(usize),
    Old(Box<CoverageFeedback>),
}

impl CoverageFeedback {
    pub fn get_block_uniqueness(&self) -> usize {
        match self {
            CoverageFeedback::NewBlock { uniqueness } => *uniqueness,
            CoverageFeedback::NoCoverage(cov) => *cov,
            _ => unreachable!(),
        }
    }

    pub fn get_edge_uniqueness(&self) -> usize {
        match self {
            CoverageFeedback::NewEdge { uniqueness } => *uniqueness,
            CoverageFeedback::NoCoverage(cov) => *cov,
            _ => unreachable!(),
        }
    }

    pub(crate) fn seed_post_fix(&self) -> String {
        match self {
            CoverageFeedback::NewBlock { uniqueness } => format!("block_{}", uniqueness),
            CoverageFeedback::NewEdge { uniqueness } => format!("edge_{}", uniqueness),
            CoverageFeedback::NewPath { uniqueness } => format!("path_{}", uniqueness),
            CoverageFeedback::NoCoverage(cov) => format!("nocov_{}", cov),
            _ => unreachable!(),
        }
    }

    pub(crate) fn from_file_name(seed_file_name: &str) -> Self {
        let parts: Vec<&str> = seed_file_name.split('_').collect();
        if parts.len() < 3 {
            panic!("Invalid seed file name: {}", seed_file_name);
        }
        let cov_type = parts[1];
        let uniqueness = parts[2].parse::<usize>().unwrap_or(0);
        let cov = match cov_type {
            "block" => CoverageFeedback::NewBlock { uniqueness },
            "edge" => CoverageFeedback::NewEdge { uniqueness },
            "path" => CoverageFeedback::NewPath { uniqueness },
            "nocov" => CoverageFeedback::NoCoverage(uniqueness),
            _ => unreachable!(),
        };
        CoverageFeedback::Old(Box::new(cov))
    }

    fn priority(&self) -> usize {
        match self {
            CoverageFeedback::NewBlock { .. } => 5,
            CoverageFeedback::NewEdge { .. } => 4,
            CoverageFeedback::NewPath { .. } => 3,
            CoverageFeedback::NoCoverage(_) => 2,
            CoverageFeedback::Old(_) => 1,
        }
    }
}

impl PartialOrd for CoverageFeedback {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (
                CoverageFeedback::NewBlock { uniqueness: u1 },
                CoverageFeedback::NewBlock { uniqueness: u2 },
            ) => Some(u2.cmp(u1)),
            (
                CoverageFeedback::NewEdge { uniqueness: u1 },
                CoverageFeedback::NewEdge { uniqueness: u2 },
            ) => Some(u2.cmp(u1)),
            (
                CoverageFeedback::NewPath {
                    uniqueness: u1,
                },
                CoverageFeedback::NewPath {
                    uniqueness: u2,
                },
            ) => Some(u2.cmp(u1)),
            (CoverageFeedback::NoCoverage(cov1), CoverageFeedback::NoCoverage(cov2)) => {
                Some(cov2.cmp(cov1))
            }
            (CoverageFeedback::NoCoverage(..), _) => {
                Some(Ordering::Less)
            }
            (CoverageFeedback::Old(c1), CoverageFeedback::Old(c2)) => Some(c1.cmp(c2)),
            _ => self.priority().partial_cmp(&other.priority()),
        }
    }
}

impl Ord for CoverageFeedback {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl CoverageFeedback {
    pub fn new_cov(&self) -> bool {
        matches!(
            self,
            CoverageFeedback::NewBlock { .. }
                | CoverageFeedback::NewEdge { .. }
                | CoverageFeedback::NewPath { .. }
        )
    }
}

pub trait CoverageMetric: Send {
    /// Update coverage with the given path and return true if new coverage was found
    /// along with a score
    fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback;

    /// Get the coverage information
    fn cov_info(&self) -> Value;

    fn full_cov(&self) -> Value {
        Value::Null
    }

    /// Get the name of the metric
    fn name(&self) -> &'static str {
        ""
    }

    fn priority(&self) -> usize {
        0
    }
}

/// Get a coverage metric by name
pub fn get_coverage_metric_by_name(name: &str) -> Option<Box<dyn CoverageMetric>> {
    match name {
        "block" => Some(Box::new(BlockCoverage::default())),
        "edge" => Some(Box::new(EdgeCoverage::default())),
        "path" => Some(Box::new(PathCoverage::default())),
        "pfp" => Some(Box::new(PerFunctionPathCoverage::default())),
        "rawpath" => Some(Box::new(RawPathCoverage::default())),
        _ => None,
    }
}

#[cached]
pub fn get_metric_priority(name: String) -> usize {
    get_coverage_metric_by_name(&name)
        .map(|m| m.priority())
        .unwrap()
}

pub type CoverageFeedbacks<'a> = BTreeMap<&'a str, CoverageFeedback>;

/// Track multiple coverage metrics simultaneously
#[derive(Default)]
pub struct CoverageMetricAggregator {
    metrics: Vec<Box<dyn CoverageMetric>>,
}

impl CoverageMetricAggregator {
    pub fn new(metrics: Vec<Box<dyn CoverageMetric>>) -> Self {
        Self { metrics }
    }

    pub fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedbacks<'static> {
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

    pub fn full_cov(&self) -> BTreeMap<&'static str, Value> {
        let mut full_covs = BTreeMap::new();
        for metric in &self.metrics {
            let full_cov = metric.full_cov();
            if !full_cov.is_null() {
                full_covs.insert(metric.name(), full_cov);
            }
        }
        full_covs
    }
}
