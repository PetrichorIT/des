use std::{fmt::Debug, str::FromStr};

use log::LevelFilter;

const ENV_RUST_LOG: &str = "RUST_LOG";

#[derive(Debug, Clone)]
pub(super) struct FilterPolicy {
    filters: Vec<(Capture, LevelFilter)>,
}

#[derive(Clone)]
struct Capture {
    parts: Vec<CaptureParts>,
}

// module.*.path.**.node*

#[derive(Clone)]
enum CaptureParts {
    AnySegmentRegex,
    AnyPathRegex,
    Path(Vec<String>),
}

impl Capture {
    fn matches(&self, s: &str) -> bool {
        let comps = s.split('.').collect::<Vec<_>>();
        let mut i = 0; // comp ptr
        let mut j = 0; // rule ptr
        while j < self.parts.len() {
            if i >= comps.len() {
                return false;
            }
            match &self.parts[j] {
                CaptureParts::AnySegmentRegex => {
                    j += 1;
                    i += 1;
                }
                CaptureParts::AnyPathRegex => todo!(),
                CaptureParts::Path(rules) => {
                    if matches_path(comps[i], rules) {
                        i += 1;
                        j += 1;
                    } else {
                        return false;
                    }
                }
            }
        }
        i == comps.len()
    }
}

fn matches_path(s: &str, rules: &[String]) -> bool {
    matches_path_front(s, rules)
}

fn matches_path_front(s: &str, rules: &[String]) -> bool {
    if let Some(mut rem) = s.strip_prefix(&rules[0]) {
        if rules.len() == 1 {
            rem.is_empty()
        } else {
            // This may only occur if this rule is the last one
            if rules[1] == "" {
                debug_assert_eq!(rules.len(), 2);
                return true;
            }
            // Strip n bytes via * an search for submatch
            while let Some(idx) = rem.find(&rules[1]) {
                rem = &rem[idx..];
                if matches_path_front(rem, &rules[1..]) {
                    return true;
                } else {
                    let pat = rules[1].len();
                    rem = &rem[pat..];
                }
            }
            false
        }
    } else {
        false
    }
}

impl FromStr for Capture {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components = s.split('.');
        let mut parts = Vec::new();
        for component in components {
            if component.is_empty() {
                break;
            }

            if component == "**" {
                parts.push(CaptureParts::AnyPathRegex);
                continue;
            }

            if component == "*" {
                parts.push(CaptureParts::AnySegmentRegex);
                continue;
            }

            let split = component
                .split('*')
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            parts.push(CaptureParts::Path(split))
        }

        Ok(Self { parts })
    }
}

impl Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..self.parts.len() {
            write!(f, "{:?}", self.parts[i])?;
            if i != self.parts.len() - 1 {
                write!(f, ".")?;
            }
        }
        Ok(())
    }
}

impl Debug for CaptureParts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnyPathRegex => write!(f, "**"),
            Self::AnySegmentRegex => write!(f, "*"),
            Self::Path(parts) => write!(f, "{}", parts.join("*")),
        }
    }
}
impl FilterPolicy {
    /// Creates a new filter policy from env
    pub(super) fn new(parse_env: bool) -> Self {
        let mut this = FilterPolicy {
            filters: Vec::new(),
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

            let capture = Capture::from_str(parts[0]).unwrap();

            let filter = match parts[1].to_lowercase().as_str() {
                "trace" => LevelFilter::Trace,
                "debug" => LevelFilter::Debug,
                "info" => LevelFilter::Info,
                "warn" => LevelFilter::Warn,
                "error" => LevelFilter::Error,
                _ => continue,
            };

            self.filters.push((capture, filter))
        }
    }

    /// Constructs a filter from the
    pub(super) fn filter_for(&self, s: &str, base: LevelFilter) -> LevelFilter {
        let mut current = base;
        for (capture, filter) in &self.filters {
            if capture.matches(s) {
                current = current.min(*filter)
            }
        }
        current
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Capture;

    #[test]
    fn captures_static_path() {
        let cap = Capture::from_str("a.b.ccc.d").unwrap();
        assert_eq!(cap.matches("a.b.ccc.d"), true);
        assert_eq!(cap.matches("a.b.ccc.d.e"), false);
        assert_eq!(cap.matches("a.b.cc.d"), false);
        assert_eq!(cap.matches("a.bd.ccc.d"), false);
        assert_eq!(cap.matches("a.b.ccc.dd"), false);

        let cap = Capture::from_str("a_b_c_d_e_f.a").unwrap();
        assert_eq!(cap.matches("a_b_c_d_e_f.a"), true);
        assert_eq!(cap.matches("a_b_c_d_e_f_.a"), false);
        assert_eq!(cap.matches("a_b_c_d_e_f.a.g"), false);
    }

    #[test]
    fn captures_path_with_any_segments() {
        let cap = Capture::from_str("a.*.ccc.d").unwrap();
        assert_eq!(cap.matches("a.b.ccc.d"), true);
        assert_eq!(cap.matches("a.b.ccc.d.e"), false);
        assert_eq!(cap.matches("a.b.cc.d"), false);
        assert_eq!(cap.matches("a.bd.ccc.d"), true);
        assert_eq!(cap.matches("a.b.ccc.dd"), false);
        assert_eq!(cap.matches("a.efgb.ccc.d"), true);
        assert_eq!(cap.matches("a.ef.gb.ccc.d"), false);

        let cap = Capture::from_str("*.a.b.c").unwrap();
        assert_eq!(cap.matches("a.a.b.c"), true);
        assert_eq!(cap.matches("bbbb.a.b.c"), true);
        assert_eq!(cap.matches("a.b.c"), false);
        assert_eq!(cap.matches("a.a.a.b.c"), false);

        let cap = Capture::from_str("a.b.c.*").unwrap();
        assert_eq!(cap.matches("a.b.c"), false);
        assert_eq!(cap.matches("a.b.c.defg"), true);
        assert_eq!(cap.matches("a.b.c.def.g"), false);
    }

    #[test]
    fn captures_path_with_mixed_segments() {
        let cap = Capture::from_str("a.*b.c").unwrap();
        assert_eq!(cap.matches("a.b.c"), true);
        assert_eq!(cap.matches("a.123b.c"), true);
        assert_eq!(cap.matches("a.bb.c"), true);
        assert_eq!(cap.matches("a.x.c"), false);
        assert_eq!(cap.matches("a.bd.c"), false);

        let cap = Capture::from_str("a.b*.c").unwrap();
        assert_eq!(cap.matches("a.b.c"), true);
        assert_eq!(cap.matches("a.b123.c"), true);
        assert_eq!(cap.matches("a.bb.c"), true);
        assert_eq!(cap.matches("a.x.c"), false);
        assert_eq!(cap.matches("a.db.c"), false);

        let cap = Capture::from_str("a.*b*.c").unwrap();
        assert_eq!(cap.matches("a.b.c"), true);
        assert_eq!(cap.matches("a.123b.c"), true);
        assert_eq!(cap.matches("a.bb.c"), true);
        assert_eq!(cap.matches("a.x.c"), false);
        assert_eq!(cap.matches("a.bd.c"), true); // C

        assert_eq!(cap.matches("a.b.c"), true);
        assert_eq!(cap.matches("a.b123.c"), true);
        assert_eq!(cap.matches("a.bb.c"), true);
        assert_eq!(cap.matches("a.x.c"), false);
        assert_eq!(cap.matches("a.db.c"), true); // C
    }
}
