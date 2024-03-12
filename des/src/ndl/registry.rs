use fxhash::{FxBuildHasher, FxHashMap};
use std::fmt;

use crate::{
    net::processing::{IntoProcessingElements, ProcessingElements},
    prelude::{Module, ObjectPath},
};

/// A type that can be created based on the nodes path and a
/// NDL symbol.
///
/// This trait is used in combination with the [`registry`](crate::registry)
/// macro, when creating a software registry. Note that this type
/// is automatically derived for any type that implements [`Default`].
pub trait RegistryCreatable {
    /// Creates a instance of `Self` from a path and symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::ndl::*;
    /// # use des::registry;
    /// # fn extract_dns_from_oid(path: &ObjectPath) {}
    /// struct Server { /* ... */ }
    ///
    /// impl RegistryCreatable for Server {
    ///     fn create(path: &ObjectPath, symbol: &str) -> Self {
    ///         let dns_name = extract_dns_from_oid(path);
    ///         Self { /* ... */ }
    ///     }
    /// }
    ///
    /// impl Module for Server {
    ///     /* ... */
    /// }
    ///
    /// # return;
    /// let mut sim = Sim::ndl(
    ///     "path/to/ndl",
    ///     registry![Server, else _]
    /// );
    /// ```
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
#[must_use]
pub struct Registry {
    symbols: FxHashMap<String, Box<GenByObjectPath>>,
    custom: Vec<Box<GenByObjectPathAndSymbol>>,
    fallback: Option<Box<Fallback>>,
}

type GenByObjectPath = dyn Fn(&ObjectPath) -> ProcessingElements;
type GenByObjectPathAndSymbol = dyn Fn(&ObjectPath, &str) -> Option<ProcessingElements>;
type Fallback = dyn Fn() -> ProcessingElements;

impl Registry {
    /// Creates a new empty registry.
    ///
    /// Registrys can be populated using the builder pattern or using
    /// the [`registry`](crate::registry) macro.
    ///
    /// # Examples
    ///
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::ndl::*;
    /// # struct HostModule;
    /// # impl Module for HostModule {}
    /// let registry = Registry::new()
    ///     .symbol("Host", |_| HostModule)
    ///     .custom(|path, symbol| {
    ///         /* ... */
    ///         # Option::<HostModule>::None
    ///     })
    ///     .with_default_fallback();
    /// ```
    ///
    /// Or create a registry using macros. Note that the macro returns a `Registry` so
    /// the builder pattner can still be used. Note that types passed to the macro
    /// **must** implement the `RegistryCreatable` trait.
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::ndl::*;
    /// # use des::registry;
    /// # #[derive(Default)]
    /// # struct HostModule;
    /// # impl Module for HostModule {}
    /// type Host = HostModule; // symbol names and type names must match
    /// let registry = registry![Host].with_default_fallback();
    /// ```
    pub fn new() -> Registry {
        Self {
            symbols: FxHashMap::with_hasher(FxBuildHasher::default()),
            custom: Vec::new(),
            fallback: None,
        }
    }

    /// Sets the `DefaultFallbackModule` as a fallback.
    ///
    /// The default fallback module will behave as follows:
    /// - `at_sim_start` and `àt_sim_end` are NOP with 1 sim start stage
    /// - `reset` is a NOP
    /// - `handle_message` will log any incoming messages to stderr
    /// - `DefaultFallbackModule` is **not** async
    /// - `DefaulfFallbackModule` will **not** load common plugins
    ///
    /// See [`Registry::with_fallback`] for more infomation.
    pub fn with_default_fallback(self) -> Self {
        self.with_fallback(|| DefaultFallbackModule)
    }

    /// Sets a fallback module, that will be used if no other
    /// directive matched.
    ///
    /// # Why have a fallback module ?
    ///
    /// By their very nature fallback modules all share the same software.
    /// Additionally the `fallback` function does not take any parameters
    /// so any fallback solution should not be dependent on any parameters.
    ///
    /// Accordingly fallback modules rarely have any meaningful software
    /// in them, they rather are just dummys for nodes that are not expected
    /// to receive any message. Such nodes often occur as structual elements in
    /// topologies to encapsualte certain subtopologies. E.g. a NDL topology
    /// may include a module `LAN` that internally contains hosts, switches and routers.
    /// All internal components will have meeaninful software attached, but the `LAN` object
    /// itself should not, since it only exposes gate chains from child modules. In this
    /// case `LAN` will never receive any messages itself, so a fallback dummy can be used.
    ///
    /// Fallback modules should be used with care, since the existence of a fallback
    /// solution within a registry implies, that any given topology can be
    /// populated with this registry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::ndl::*;
    /// # use des::prelude::*;
    /// # struct HostModule;
    /// # impl Module for HostModule {}
    /// # struct SwitchModule;
    /// # impl Module for SwitchModule {}
    /// # struct RouterModule;
    /// # impl Module for RouterModule {}
    /// # struct NOP;
    /// # impl Module for NOP {}
    /// let registry = Registry::new()
    ///     .symbol("Host", |_| HostModule)
    ///     .symbol("Switch", |_| SwitchModule)
    ///     .symbol("Router", |_| RouterModule)
    ///     .with_fallback(|| NOP);
    ///
    /// # return;
    /// let mut sim = Sim::ndl("path/to/ndl", registry);
    /// /* ... */
    /// ```
    pub fn with_fallback<M: Module>(mut self, fallback: impl Fn() -> M + 'static) -> Self {
        self.fallback = Some(Box::new(move || fallback().to_processing_chain()));
        self
    }

    /// Adds a symbol mapping to the registry.
    ///
    /// All nodes with the symbol `ty` will now use the provided genertator
    /// function `f` to generate software. This function is thus only parameterized
    /// by the objects path.
    ///
    /// Newer assigments will override older assigments to the same symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::ndl::*;
    /// # use des::prelude::*;
    /// struct Host { /* ... */ }
    ///
    /// impl Host {
    ///     fn new(path: &ObjectPath) -> Self {
    ///         /* .. */
    /// #       Host {}
    ///     }
    /// }
    ///
    /// impl Module for Host {
    ///     /* ... */
    /// }
    ///
    /// /* ... */
    /// # struct Router;
    /// # impl Router { fn new(path: &ObjectPath) -> Self { Self } }
    /// # impl Module for Router {}
    ///
    /// let registry = Registry::new()
    ///     .symbol("Host", |path| Host::new(path))
    ///     .symbol("Router", |path| Router::new(path));
    ///
    /// # return;
    /// let mut sim = Sim::ndl("path/to/ndl", registry);
    /// /* ... */
    /// ```
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

    /// Adds a custom directive to the registry.
    ///
    /// A custom directive is a function that optionally returns a module.
    /// If `None` is returned that indicates that the directive is not responsible for
    /// generating software for this node. Custom directive are executed in order
    /// of definition.
    ///
    /// Note that a custom directive that allways returns `Some(...)` is equivalent
    /// to a fallback module.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// # use des::ndl::*;
    /// # struct GoogleGeneralHost;
    /// # impl GoogleGeneralHost { fn new(_: &ObjectPath) -> Self { Self }}
    /// # impl Module for GoogleGeneralHost {}
    /// let symbols = ["Host", "Client", "Server"];
    ///
    /// let registry = Registry::new()
    ///     .custom(move |path, symbol| {
    ///         if symbols.contains(&symbol) {
    ///             if path.as_str().starts_with("google") {
    ///                 Some(GoogleGeneralHost::new(path))
    ///             } else {
    ///                 None
    ///             }
    ///         } else {
    ///             None
    ///         }
    ///     });
    ///
    /// # return;
    /// let mut sim = Sim::ndl("path/to/ndl", registry);
    /// /* ... */
    /// ```
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
    pub(super) fn lookup(&self, path: &ObjectPath, ty: &str) -> Option<ProcessingElements> {
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

impl AsRef<Registry> for Registry {
    fn as_ref(&self) -> &Registry {
        self
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
    fn stack(&self) -> impl IntoProcessingElements {}

    fn handle_message(&mut self, msg: crate::prelude::Message) {
        tracing::error!(
            ?msg,
            "received message: fallback dummy should never receive any messages"
        );
    }
}
