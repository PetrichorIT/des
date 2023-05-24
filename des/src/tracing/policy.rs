use std::{fmt::Debug, io::stdout};

use super::{
    format::{ColorfulTracingFormatter, TracingFormatter},
    output::TracingOutput,
};

/// A configuration for a single scope, defining the output
/// and formatting behaviour of the scope.
pub struct ScopeConfiguration {
    /// The output device for the scope.
    pub output: Box<dyn TracingOutput>,
    /// A formatter used to write to the output device.
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

/// A policy for configuring loggin scopes
pub trait ScopeConfigurationPolicy {
    /// Creates a new cfg for a new scope.
    fn configure(&self, scope: &str) -> ScopeConfiguration;
}

impl<T: Fn(&str) -> ScopeConfiguration> ScopeConfigurationPolicy for Box<T> {
    fn configure(&self, scope: &str) -> ScopeConfiguration {
        self(scope)
    }
}

/// A configuration policy that applies the default scope configuration
/// to all scopes.
#[derive(Debug)]
pub struct DefaultScopeConfigurationPolicy;
impl ScopeConfigurationPolicy for DefaultScopeConfigurationPolicy {
    fn configure(&self, _: &str) -> ScopeConfiguration {
        ScopeConfiguration::default()
    }
}
