use super::{CoverageFeedback, CoverageMetric};
use path_reduction::json_parser::parse_json_file;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

type BlockID = u32;
pub struct FunCoverage {
    funs: FxHashMap<u32, usize>,
    first_map: FxHashSet<BlockID>,
}

impl Default for FunCoverage {
    fn default() -> Self {
        let cfg_file = std::env::var("CFG_FILE").unwrap_or_default();
        Self::from_json(&cfg_file)
    }
}

impl FunCoverage {
    fn from_json(path: &str) -> Self {
        let modules = parse_json_file(path).unwrap();
        let first_to_lasts = modules
            .iter()
            .flat_map(|module| {
                module
                    .functions
                    .iter()
                    .map(|func| func.entry_block)
            })
            .collect();
        Self {
            funs: FxHashMap::default(),
            first_map: first_to_lasts,
        }
    }
}

impl CoverageMetric for FunCoverage {
    fn update_from_path(&mut self, path: &[u32]) -> CoverageFeedback {
        let mut new_cov = false;
        let mut uniq = usize::MAX;
        for block in path {
            if self.first_map.contains(block) {
                let count = *self
                    .funs
                    .entry(*block)
                    .and_modify(|count| *count += 1)
                    .or_insert_with(|| {
                        new_cov = true;
                        1
                    });
                uniq = uniq.min(count);
            }
        }
        if new_cov {
            CoverageFeedback::NewBlock { uniqueness: uniq }
        } else {
            CoverageFeedback::NoCoverage(uniq)
        }
    }

    fn cov_info(&self) -> Value {
        Value::Number(self.funs.len().into())
    }

    // an array of [from, to, count]
    fn full_cov(&self) -> Value {
        Value::Array(
            self.funs
                .iter()
                .map(|(fun_id, _)| Value::Number((*fun_id).into()))
                .collect(),
        )
    }

    fn name(&self) -> &'static str {
        "fun"
    }

    fn priority(&self) -> usize {
        90
    }
}
