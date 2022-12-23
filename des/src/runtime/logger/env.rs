use log::LevelFilter;
use std::str::FromStr;

pub(super) struct LogEnvOptions {
    max_level: LevelFilter,
    level_overrides: Vec<(String, LevelFilter)>,
}

impl LogEnvOptions {
    pub(super) fn new() -> Self {
        if let Ok(env) = std::env::var("RUST_LOG") {
            Self::from_str(&env).unwrap()
        } else {
            Self::default()
        }
    }

    pub(super) fn level_filter_for(&self, target: &str) -> LevelFilter {
        let mut level = self.max_level;
        for (trg, lv) in &self.level_overrides {
            if matches(target, trg) {
                level = *lv;
            }
        }
        level
    }
}

fn matches(path: &str, rule: &str) -> bool {
    let path = path.split('.').collect::<Vec<_>>();
    let rule = rule.split('.').collect::<Vec<_>>();

    for (path_fragment, regex) in path.iter().zip(&rule) {
        match *regex {
            "**" => return true,
            "*" => continue,
            regex if regex == *path_fragment => continue,
            _ => return false,
        }
    }

    rule.len() == path.len()
}

impl FromStr for LogEnvOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // split into seperate expresions
        let expr = s.split(',').collect::<Vec<_>>();

        // get first expr
        let max_level = if let Some(lv) = expr.first() {
            match lv.parse::<LevelFilter>() {
                Ok(v) => v,
                Err(e) => return Err(format!("Could not parse enviroment var RUST_LOG: {e}")),
            }
        } else {
            return Err(
                "Could not parse enviroment var RUST_LOG: Did not provide default log level"
                    .to_string(),
            );
        };

        let mut level_overrides = Vec::new();
        for ovr in expr.iter().skip(1) {
            let split = ovr.split(':').collect::<Vec<_>>();
            if split.len() != 2 {
                return Err(format!(
                    "Could not parse enviroment var RUST_LOG: Overrides '{ovr}' was formatted wrong"
                ));
            }

            let level = match split[1].parse::<LevelFilter>() {
                Ok(v) => v,
                Err(e) => return Err(format!("Could not parse enviroment var RUST_LOG: {e}")),
            };

            level_overrides.push((split[0].to_string(), level));
        }

        Ok(Self {
            max_level,
            level_overrides,
        })
    }
}

impl Default for LogEnvOptions {
    fn default() -> Self {
        Self {
            max_level: log::STATIC_MAX_LEVEL,
            level_overrides: Vec::new(),
        }
    }
}
