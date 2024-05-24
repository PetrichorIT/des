use std::{fmt, marker::PhantomData};

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
    /// This function if called by [`Sim::ndl`](crate::net::Sim) will be called
    /// within node-context.
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

#[doc(hidden)]
pub trait RegistryCreatableInner {
    type Target;
    fn create_inner(&mut self, path: &ObjectPath, symbol: &str) -> Self::Target;
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
pub struct Registry<L: Layer> {
    layer: L,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct EmptyLayer;

#[doc(hidden)]
#[derive(Debug)]
pub struct SymbolLayer<L, M>
where
    L: Layer,
    M: RegistryCreatableInner,
    M::Target: Module,
{
    ty: String,
    inner: L,
    factory: M,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct FallbackLayer<L: Layer, F: Fn() -> M, M: Module> {
    f: F,
    inner: L,
}

impl Registry<EmptyLayer> {
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
    /// # #[derive(Default)]
    /// # struct HostModule;
    /// # impl Module for HostModule {}
    /// let registry = Registry::new()
    ///     .symbol::<HostModule>("Host")
    ///     .symbol_fn("OtherHost", |pat| {
    ///         /* ... */
    ///         # HostModule
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
    pub fn new() -> Registry<EmptyLayer> {
        Registry { layer: EmptyLayer }
    }
}

impl<L: Layer> Registry<L> {
    pub(super) fn resolve(
        &mut self,
        path: &ObjectPath,
        symbol: &str,
    ) -> Option<ProcessingElements> {
        self.layer.resolve(path, symbol)
    }

    /// Adds a symbol mapping to the registry.
    ///
    /// All nodes with the symbol `ty` will now use the provided genertator
    /// type `M: RegistryCreatable` to generate software. This function is thus only
    /// parameterized by the objects path.
    ///
    /// Newer assigments will override older assigments to the same symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::ndl::*;
    /// # use des::prelude::*;
    /// # #[derive(Default)]
    /// struct Host { /* ... */ }
    ///
    ///
    /// impl Module for Host {
    ///     /* ... */
    /// }
    ///
    ///
    /// /* ... */
    /// # #[derive(Default)]
    /// # struct Router;
    /// # impl Router { fn new(path: &ObjectPath) -> Self { Self } }
    /// # impl Module for Router {}
    ///
    /// let registry = Registry::new()
    ///     .symbol::<Host>("Host")
    ///     .symbol::<Router>("Router");
    ///
    /// # return;
    /// let mut sim = Sim::ndl("path/to/ndl", registry);
    /// /* ... */
    /// ```
    pub fn symbol<M>(self, ty: impl AsRef<str>) -> Registry<SymbolLayer<L, Create<M>>>
    where
        M: RegistryCreatable + Module,
    {
        assert!(
            !L::FINAL,
            "cannot add another layer, the registry was finalized by the previous one"
        );
        Registry {
            layer: SymbolLayer {
                ty: ty.as_ref().to_string(),
                inner: self.layer,
                factory: Create {
                    _phantom: PhantomData,
                },
            },
        }
    }

    /// Adds a custom symbol mapping to the registry.
    ///
    /// This mapping uses the provided clousure to generate types, instead of `RegistryCreatable`.
    /// This allows for the assignment of multiple types to one symbol, using Box<dyn Module> as
    /// a return type.
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
    /// # type AwsGeneralHost = GoogleGeneralHost;
    /// let mut google_nodes = 0;
    ///
    /// let registry = Registry::new()
    ///     .symbol_fn("Host", |path| {
    ///         if path.as_str().starts_with("google") {
    ///             google_nodes += 1;
    ///         }
    ///         GoogleGeneralHost::new(path)
    ///     });
    ///
    /// # return;
    /// let mut sim = Sim::ndl("path/to/ndl", registry);
    /// /* ... */
    /// ```
    pub fn symbol_fn<F, M>(
        self,
        ty: impl AsRef<str>,
        mut f: F,
    ) -> Registry<SymbolLayer<L, FnWrapper<impl FnMut(&ObjectPath, &str) -> M, M>>>
    where
        F: FnMut(&ObjectPath) -> M,
        M: Module,
    {
        Registry {
            layer: SymbolLayer {
                ty: ty.as_ref().to_string(),
                inner: self.layer,
                factory: FnWrapper {
                    f: move |path, _| f(path),
                },
            },
        }
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
    /// # #[derive(Default)]
    /// # struct HostModule;
    /// # impl Module for HostModule {}
    /// # #[derive(Default)]
    /// # struct SwitchModule;
    /// # impl Module for SwitchModule {}
    /// # #[derive(Default)]
    /// # struct RouterModule;
    /// # impl Module for RouterModule {}
    /// # #[derive(Default)]
    /// # struct NOP;
    /// # impl Module for NOP {}
    /// let registry = Registry::new()
    ///     .symbol::<HostModule>("Host")
    ///     .symbol::<SwitchModule>("Switch")
    ///     .symbol::<RouterModule>("Router")
    ///     .with_fallback(|| NOP);
    ///
    /// # return;
    /// let mut sim = Sim::ndl("path/to/ndl", registry);
    /// /* ... */
    /// ```
    pub fn with_fallback<F, M>(self, f: F) -> Registry<FallbackLayer<L, F, M>>
    where
        F: Fn() -> M,
        M: Module,
    {
        assert!(
            !L::FINAL,
            "cannot add another layer, the registry was finalized by the previous one"
        );
        Registry {
            layer: FallbackLayer {
                f,
                inner: self.layer,
            },
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
    pub fn with_default_fallback(
        self,
    ) -> Registry<FallbackLayer<L, impl Fn() -> DefaultFallbackModule, DefaultFallbackModule>> {
        self.with_fallback(|| DefaultFallbackModule)
    }
}

impl<L: Layer> AsRef<Registry<L>> for Registry<L> {
    fn as_ref(&self) -> &Registry<L> {
        self
    }
}

impl<L: Layer> AsMut<Registry<L>> for Registry<L> {
    fn as_mut(&mut self) -> &mut Registry<L> {
        self
    }
}

impl<L: Layer> fmt::Debug for Registry<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry").finish()
    }
}

#[doc(hidden)]
pub trait Layer {
    const FINAL: bool = false;
    #[doc(hidden)]
    fn resolve(&mut self, path: &ObjectPath, symbol: &str) -> Option<ProcessingElements>;
}

impl Layer for EmptyLayer {
    fn resolve(&mut self, _: &ObjectPath, _: &str) -> Option<ProcessingElements> {
        None
    }
}

impl<L, M> Layer for SymbolLayer<L, M>
where
    M: RegistryCreatableInner,
    M::Target: Module,
    L: Layer,
{
    fn resolve(&mut self, path: &ObjectPath, symbol: &str) -> Option<ProcessingElements> {
        self.inner.resolve(path, symbol).or_else(|| {
            if symbol == self.ty {
                Some(
                    self.factory
                        .create_inner(path, symbol)
                        .to_processing_chain(),
                )
            } else {
                None
            }
        })
    }
}

impl<L, F, M> Layer for FallbackLayer<L, F, M>
where
    F: Fn() -> M,
    M: Module,
    L: Layer,
{
    const FINAL: bool = true;
    fn resolve(&mut self, path: &ObjectPath, symbol: &str) -> Option<ProcessingElements> {
        Some(
            self.inner
                .resolve(path, symbol)
                .unwrap_or_else(|| (self.f)().to_processing_chain()),
        )
    }
}

#[doc(hidden)]
#[derive(Debug, Default)]
pub struct DefaultFallbackModule;
impl Module for DefaultFallbackModule {
    fn stack(&self) -> impl IntoProcessingElements {}
    fn handle_message(&mut self, msg: crate::prelude::Message) {
        tracing::error!(
            ?msg,
            "received message: fallback dummy should never receive any messages"
        );
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Create<M: RegistryCreatable + Module> {
    _phantom: PhantomData<M>,
}

impl<M: RegistryCreatable + Module> RegistryCreatableInner for Create<M> {
    type Target = M;
    fn create_inner(&mut self, path: &ObjectPath, symbol: &str) -> Self::Target {
        M::create(path, symbol)
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct FnWrapper<F: FnMut(&ObjectPath, &str) -> M, M: Module> {
    f: F,
}

impl<F: FnMut(&ObjectPath, &str) -> M, M: Module> RegistryCreatableInner for FnWrapper<F, M> {
    type Target = M;
    fn create_inner(&mut self, path: &ObjectPath, symbol: &str) -> Self::Target {
        (self.f)(path, symbol)
    }
}
