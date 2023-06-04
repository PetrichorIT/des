use fxhash::{FxBuildHasher, FxHashMap};
use std::fmt::Debug;
use tracing::metadata::LevelFilter;

const ENV_RUST_LOG: &str = "RUST_LOG";

#[derive(Debug, Clone)]
pub(super) struct TargetFilters {
    matches: FxHashMap<String, LevelFilter>,
    fallback: Vec<(String, LevelFilter)>,
}

impl TargetFilters {
    /// Creates a new filter policy from env
    pub(super) fn new(parse_env: bool) -> Self {
        let mut this = TargetFilters {
            matches: FxHashMap::with_hasher(FxBuildHasher::default()),
            fallback: Vec::new(),
        };
        if parse_env {
            let Ok(s) = std::env::var(ENV_RUST_LOG) else {
                return this
            };
            this.parse_str(&s);
        }
        this
    }

    pub(super) fn parse_str(&mut self, s: &str) {
        for part in s.split(',') {
            let parts = part.split('=').collect::<Vec<_>>();
            if parts.len() != 2 {
                continue;
            }

            let mut capture = parts[0].to_string();

            let filter = match parts[1].to_lowercase().as_str() {
                "trace" => LevelFilter::TRACE,
                "debug" => LevelFilter::DEBUG,
                "info" => LevelFilter::INFO,
                "warn" => LevelFilter::WARN,
                "error" => LevelFilter::ERROR,
                "off" => LevelFilter::OFF,
                _ => continue,
            };

            if capture.ends_with("*") {
                capture.pop();
                match self
                    .fallback
                    .binary_search_by(|e| capture.len().cmp(&e.0.len()))
                {
                    Ok(i) | Err(i) => self.fallback.insert(i, (capture, filter)),
                };
            } else {
                self.matches.insert(capture, filter);
            }
        }
    }

    /// Constructs a filter from the
    pub(super) fn filter_for(&self, s: &str, base: LevelFilter) -> LevelFilter {
        self.matches
            .get(s)
            .copied()
            .or_else(|| {
                for filter in &self.fallback {
                    if s.starts_with(&filter.0) {
                        return Some(filter.1);
                    }
                }
                None
            })
            .unwrap_or(base)
    }
}
