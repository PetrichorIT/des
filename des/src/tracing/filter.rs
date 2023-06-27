use std::{str::FromStr, error::Error, fmt::{Display, Debug}, env};
use tracing::{metadata::LevelFilter, level_filters::STATIC_MAX_LEVEL, Metadata};

use super::TracingRecord;

#[derive(Debug)]
pub(super) struct Filters {
    directives: Vec<FilterDiretive>,
}

impl Filters {
    pub(super) fn from_env() -> Result<Self, FilterDirectiveParsingError> {
        let env = env::var("RUST_LOG").unwrap_or(String::new());
        env.parse()
    }

    pub(super) fn level_filter_for(&self, record: &TracingRecord) -> LevelFilter {
        let mut lvl = STATIC_MAX_LEVEL;
        for directive in &self.directives {
            lvl = directive.level_filter_for(lvl, record);
        }
        lvl
    }

    pub(super) fn callsite_filter_for(&self, record: &Metadata) -> LevelFilter {
        let mut lvl = STATIC_MAX_LEVEL;
        for directive in &self.directives {
            lvl = directive.callsite_filter_for(lvl, record);
        }
        lvl
    }
}

impl FromStr for Filters {
    type Err = FilterDirectiveParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split(',');
        let mut directives = Vec::new();
        for directive in split {
            if directive.is_empty() {
                continue;
            }

            directives.push(FilterDiretive::from_str(directive.trim())?);
        }

        Ok(Self {
            directives
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FilterDiretive {
    scope: Option<String>,
    target: Option<String>,
    span: Option<String>,
    fields: Option<SpanFields>,
    level: LevelFilter,
}

impl FilterDiretive {
    fn level_filter_for(&self, lvl: LevelFilter, record: &TracingRecord) -> LevelFilter {
        if let Some(ref scope) = self.scope {
            if !record.scope.map(|s| s.starts_with(scope)).unwrap_or(false) {
                return lvl;
            }
        }
       
        if let Some(ref target) = self.target {
            if !record.target.starts_with(target) {
                return lvl;
            }
        }

        if let Some(ref span) = self.span {
            if !record.spans.iter().any(|s| s.name == span){
                return lvl;
            }
        }

        if let Some(ref fields) = self.fields { 
            if !record.spans.iter().any(|s| s.fields.get(fields.key.as_str()) == Some(&fields.value)) {
                return lvl;
            }
        }

        self.level
    }

    fn callsite_filter_for(&self, lvl: LevelFilter, callsite: &Metadata) -> LevelFilter {
        if let Some(ref target) = self.target {
            if !callsite.module_path().map_or(false, |t| t.starts_with(target)) {
                return lvl;
            }
        }

        self.level
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpanFields {
    key: String,
    value: String,
}

impl FromStr for FilterDiretive {
    type Err = FilterDirectiveParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((mut s, lvl)) = s.rsplit_once('=') else {
            return Ok(FilterDiretive { 
                scope: None,
                target: None, 
                span: None, 
                fields: None, 
                level: parse_level(s)?
            });
        };

        let level = parse_level(lvl)?;

        let Some((scope_or_target, c)) = read_until_set(&mut s, &['/', '[']) else {
            return Ok(FilterDiretive {
                scope: None,
                target: Some(s.to_string()),
                span: None,
                fields: None,
                level
            });
        };

        let (target, scope) = if c == '/' {
            let scope = nonempty_or_none(scope_or_target);

            let Some(target) = read_until(&mut s, '[') else {            
                return Ok(FilterDiretive {
                    scope,
                    target: nonempty_or_none(s),
                    span: None,
                    fields: None,
                    level
                });
            };

            assert!(s.ends_with(']'));
            s = s.trim_end_matches("]");

            (nonempty_or_none(target), scope)
        } else {
            assert!(s.ends_with(']'));
            s = s.trim_end_matches("]");
            (nonempty_or_none(scope_or_target), None)
        };

        let Some(span) = read_until(&mut s, '{') else {
            return Ok(FilterDiretive { 
                scope,
                target,
                span: Some(s.to_string()), 
                fields: None, 
                level 
            });
        };

        assert!(s.ends_with('}'));
        s = s.trim_end_matches("}");
        let span = if span.is_empty() { None } else { Some(span.to_string()) };

        Ok(FilterDiretive { 
            scope,
            target,
            span, 
            fields: Some(SpanFields::from_str(s)?), 
            level
        })
    }
}


impl FromStr for SpanFields {
    type Err = FilterDirectiveParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((l, mut r)) = s.split_once('=') else {
            return Err(FilterDirectiveParsingError::MissingKeyValue)
        };
        assert!(r.starts_with('"'));
        assert!(r.ends_with('"'));

        r = r.trim_start_matches('"');
        r = r.trim_end_matches('"');

        Ok(Self { key: l.to_string(), value: r.to_string() })
    }
}

fn parse_level(lvl: &str) -> Result<LevelFilter, FilterDirectiveParsingError> {
    match lvl.to_lowercase().as_str() {
        "error" => Ok(LevelFilter::ERROR),
        "warn" => Ok(LevelFilter::WARN),
        "info" => Ok(LevelFilter::INFO),
        "debug" => Ok(LevelFilter::DEBUG),
        "trace" => Ok(LevelFilter::TRACE),
        "off" => Ok(LevelFilter::OFF),
        _ => return Err(FilterDirectiveParsingError::InvalidFilterLevel)
    }
}

fn read_until<'a>(s: &mut &'a str, p: char) -> Option<&'a str> {
    let mut offset = 0;
    for c in s.chars() {
        if c == p {
            let fwd = &s[..offset];
            *s = &s[(offset + c.len_utf8())..];
            return Some(fwd);
        }
        offset += c.len_utf8()
    }

    None
}

fn read_until_set<'a>(s: &mut &'a str, p: &[char]) -> Option<(&'a str,char)> {
    let mut offset = 0;
    for c in s.chars() {
        if p.contains(&c) {
            let fwd = &s[..offset];
            *s = &s[(offset + c.len_utf8())..];
            return Some((fwd, c));
        }
        offset += c.len_utf8()
    }

    None
}

fn nonempty_or_none(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[derive(Debug, Clone)]
pub(super) enum FilterDirectiveParsingError {
    InvalidFilterLevel,
    MissingKeyValue
}

impl Error for FilterDirectiveParsingError {}

impl Display for FilterDirectiveParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(&self, f)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() -> Result<(), FilterDirectiveParsingError> {
        assert_eq!(parse_level("info")?, LevelFilter::INFO);
        assert_eq!(parse_level("WARN")?, LevelFilter::WARN);
        assert_eq!(parse_level("Trace")?, LevelFilter::TRACE);
        assert_eq!(parse_level("debug")?, LevelFilter::DEBUG);
        assert_eq!(parse_level("off")?, LevelFilter::OFF);
        assert_eq!(parse_level("Error")?, LevelFilter::ERROR);
        Ok(())
    }

    #[test]
    fn pure_target_filters() -> Result<(), FilterDirectiveParsingError> {
        let flt = "target=info".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { 
            scope: None,
            target: Some("target".to_string()),
            span: None,
            fields: None,
            level: LevelFilter::INFO
        });

        let flt = "with-dash=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { scope: None,
            target: Some("with-dash".to_string()),
            span: None,
            fields: None,
            level: LevelFilter::TRACE
        });
        Ok(())
    }

    #[test]
    fn pure_span_filters() -> Result<(), FilterDirectiveParsingError> {
        let flt = "[spanname]=info".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive {
            scope: None,
            target: None,
            span: Some("spanname".to_string()),
            fields: None,
            level: LevelFilter::INFO
        });

        let flt = "[with-dash]=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { scope: None,
            target: None,
            span: Some("with-dash".to_string()),
            fields: None,
            level: LevelFilter::TRACE
        });
        Ok(())
    }

    #[test]
    fn mixed_target_span_filters() -> Result<(), FilterDirectiveParsingError> {
        let flt = "target[spanname]=info".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { scope: None,
            target: Some("target".to_string()),
            span: Some("spanname".to_string()),
            fields: None,
            level: LevelFilter::INFO
        });

        let flt = "with-[dash]=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { scope: None,
            target: Some("with-".to_string()),
            span: Some("dash".to_string()),
            fields: None,
            level: LevelFilter::TRACE
        });
        Ok(())
    }

    #[test]
    fn scope_definition() -> Result<(), FilterDirectiveParsingError> {
        let flt = "scope/target=info".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { 
            scope: Some("scope".to_string()),
            target: Some("target".to_string()),
            span: None,
            fields: None,
            level: LevelFilter::INFO
        });

        let flt = "scope/[dash]=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { 
            scope: Some("scope".to_string()),
            target: None,
            span: Some("dash".to_string()),
            fields: None,
            level: LevelFilter::TRACE
        });

        let flt = "/t[dash]=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive { 
            scope: None,
            target: Some("t".to_string()),
            span: Some("dash".to_string()),
            fields: None,
            level: LevelFilter::TRACE
        });
        Ok(())
    }
}