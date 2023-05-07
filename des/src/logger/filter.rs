use std::{collections::HashMap, fmt::Debug};

use log::LevelFilter;

const ENV_RUST_LOG: &str = "RUST_LOG";

#[derive(Debug, Clone)]
pub(super) struct TargetFilters {
    filters: HashMap<String, LevelFilter>,
}

impl TargetFilters {
    /// Creates a new filter policy from env
    pub(super) fn new(parse_env: bool) -> Self {
        let mut this = TargetFilters {
            filters: HashMap::new(),
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

            let capture = parts[0].to_string();

            let filter = match parts[1].to_lowercase().as_str() {
                "trace" => LevelFilter::Trace,
                "debug" => LevelFilter::Debug,
                "info" => LevelFilter::Info,
                "warn" => LevelFilter::Warn,
                "error" => LevelFilter::Error,
                _ => continue,
            };

            self.filters.insert(capture, filter);
        }
    }

    /// Constructs a filter from the
    pub(super) fn filter_for(&self, s: &str, base: LevelFilter) -> LevelFilter {
        self.filters.get(s).copied().unwrap_or(base)
    }
}
