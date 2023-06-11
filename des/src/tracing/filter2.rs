use std::{str::FromStr, error::Error, fmt::{Display, Debug}};
use tracing::{metadata::LevelFilter, Level};

use super::TracingRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FilterDiretive {
    target: Option<String>,
    span: Option<String>,
    fields: Option<SpanFields>,
    level: LevelFilter,
}


#[derive(Debug, Clone, PartialEq, Eq)]
struct SpanFields {
    key: String,
    value: String,
}

impl FromStr for FilterDiretive {
    type Err = FilterDirectiveParsingError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((mut s, lvl)) = s.split_once('=') else {
            return Ok(FilterDiretive { 
                target: None, 
                span: None, 
                fields: None, 
                level: parse_level(s)?
            });
        };

        let level = parse_level(lvl)?;

        let Some(target) = read_until(&mut s, '[') else {            
            return Ok(FilterDiretive {
                target: Some(s.to_string()),
                span: None,
                fields: None,
                level
            });
        };

        assert!(s.ends_with(']'));
        s = s.trim_end_matches("]");
        let target = if target.is_empty() { None } else { Some(target.to_string()) };

        let Some(span) = read_until(&mut s, '{') else {
            return Ok(FilterDiretive { 
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

#[derive(Debug, Clone)]
pub enum FilterDirectiveParsingError {
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
            target: Some("target".to_string()),
            span: None,
            fields: None,
            level: LevelFilter::INFO
        });

        let flt = "with-dash=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive {
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
            target: None,
            span: Some("spanname".to_string()),
            fields: None,
            level: LevelFilter::INFO
        });

        let flt = "[with-dash]=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive {
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
        assert_eq!(flt, FilterDiretive {
            target: Some("target".to_string()),
            span: Some("spanname".to_string()),
            fields: None,
            level: LevelFilter::INFO
        });

        let flt = "with-[dash]=trace".parse::<FilterDiretive>()?;
        assert_eq!(flt, FilterDiretive {
            target: Some("with-".to_string()),
            span: Some("dash".to_string()),
            fields: None,
            level: LevelFilter::TRACE
        });
        Ok(())
    }
}