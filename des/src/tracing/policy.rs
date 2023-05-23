use std::{fmt::Debug, io::stdout};

use super::{
    format::{ColorfulTracingFormatter, TracingFormatter},
    output::TracingOutput,
};

/// A cfg
pub struct ScopeConfiguration {
    pub output: Box<dyn TracingOutput>,
    pub fmt: Box<dyn TracingFormatter>,
}

impl Debug for ScopeConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopeConfiguration").finish()
    }
}

impl Default for ScopeConfiguration {
    fn default() -> Self {
        Self {
            output: Box::new(stdout()),
            fmt: Box::new(ColorfulTracingFormatter),
        }
    }
}

pub trait ScopeConfigurationPolicy {
    fn configure(&self, scope: &str) -> ScopeConfiguration;
}

impl<T: Fn(&str) -> ScopeConfiguration> ScopeConfigurationPolicy for Box<T> {
    fn configure(&self, scope: &str) -> ScopeConfiguration {
        self(scope)
    }
}

pub struct DefaultScopeConfigurationPolicy;
impl ScopeConfigurationPolicy for DefaultScopeConfigurationPolicy {
    fn configure(&self, _: &str) -> ScopeConfiguration {
        ScopeConfiguration::default()
    }
}
