use fxhash::{FxBuildHasher, FxHashMap};
use std::fmt;

use crate::{
    net::processing::ProcessingElements,
    prelude::{Module, ObjectPath},
};

/// A trait that describes objects that can be inserted into registry objects
pub trait RegistryCreatable {
    /// Create
    fn create(path: &ObjectPath, symbol: &str) -> Self;
}

impl<T: Default> RegistryCreatable for T {
    fn create(_: &ObjectPath, _: &str) -> Self {
        Self::default()
    }
}

/// A registry to attache user-defined software to nodes in
/// a simulation.
///
/// When creating a simulation from a NDL like structure,
/// that only defines the topological layout of the simulation,
/// user-defined software must be attached to created nodes to
/// make the setup complete.
///
/// This registry effectivly acts as a `fn (ObjectPath, Symbol) -> Module`
/// to assign software to each node that will be created. Since these
/// nodes are related to a NDL-Module the modules name is also provided
/// as a parameter.
pub struct Registry {
    symbols: FxHashMap<String, Box<dyn Fn(&ObjectPath) -> ProcessingElements>>,
    custom: Vec<Box<dyn Fn(&ObjectPath, &str) -> Option<ProcessingElements>>>,
    fallback: Option<Box<dyn Fn() -> ProcessingElements>>,
}

impl Registry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Registry {
        Self {
            symbols: FxHashMap::with_hasher(FxBuildHasher::default()),
            custom: Vec::new(),
            fallback: None,
        }
    }

    ///
    pub fn with_default_fallback(self) -> Self {
        self.with_fallback(|| DefaultFallbackModule)
    }

    ///
    pub fn with_fallback<M: Module>(mut self, fallback: impl Fn() -> M + 'static) -> Self {
        self.fallback = Some(Box::new(move || fallback().to_processing_chain()));
        self
    }

    ///
    pub fn symbol<M: Module>(
        mut self,
        ty: impl AsRef<str>,
        f: impl for<'a> Fn(&'a ObjectPath) -> M + 'static,
    ) -> Self {
        self.symbols.insert(
            ty.as_ref().to_string(),
            Box::new(move |path| f(path).to_processing_chain()),
        );
        self
    }

    ///
    pub fn custom<M: Module>(
        mut self,
        f: impl Fn(&ObjectPath, &str) -> Option<M> + 'static,
    ) -> Self {
        self.custom.push(Box::new(move |path, symbol| {
            Some(f(path, symbol)?.to_processing_chain())
        }));
        self
    }

    /// Lookup
    pub fn lookup(&self, path: &ObjectPath, ty: &str) -> Option<ProcessingElements> {
        // (0) Symbol resolve
        if let Some(resolver) = self.symbols.get(ty) {
            return Some(resolver(path));
        }

        // (1) Check custom handlers
        for handler in &self.custom {
            if let Some(resolved) = handler(path, ty) {
                return Some(resolved);
            }
        }

        // (2) Fallback
        self.fallback.as_ref().map(|fallback| fallback())
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry").finish()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct DefaultFallbackModule;
impl Module for DefaultFallbackModule {
    fn handle_message(&mut self, msg: crate::prelude::Message) {
        tracing::error!(
            ?msg,
            "received message: fallback dummy should never receive any messages"
        );
    }
}
