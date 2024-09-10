//! Integration of the Network-Description-Language (NDL).
//!
//! # What is NDL ?
//!
//! NDL is a decriptory language for defining network topologies.
//! Refer to [`des_ndl`] for more information.
//!
//! # How to use it ?
//!
//! This submodule provides an [`Sim::ndl`] that can create a simulation
//! that builsd a network based on a given topology.
//! Users can create such an application by providing the path to the
//! root file of the NDL description, and by providing a registry of modules.
//! This registry is used to link names of network nodes in NDL to associated
//! structs that implmenent [`Module`](crate::net::module::Module).
//! By proving both parameters, the application will load the topology and check
//! whether the network can be build. If not an descriptive error will be returned.
//!
//! ```
//! # use des::prelude::*;
//! # use des::ndl::*;
//! # use des::registry;
//! #[derive(Default)]
//! struct ModuleA;
//! /* ... */
//!
//! #[derive(Default)]
//! struct ModuleB;
//! /* ... */
//!
//! # impl Module for ModuleA {}
//! # impl Module for ModuleB {}
//! fn main() {
//!     # return;
//!     let app = match Sim::ndl("path/to/ndl.ndl", registry![ModuleA, ModuleB]) {
//!         Ok(v) => v,
//!         Err(e) => {
//!             println!("{e}");
//!             return;
//!         },
//!     };
//!     let rt = Builder::new().build(app);
//!     let _ = rt.run();
//! }
//! ```

use crate::{
    net::{self, channel::ChannelDropBehaviour, module::ModuleContext, ScopedSim, Sim},
    prelude::{Channel, ChannelMetrics, ModuleRef, ObjectPath},
    time::Duration,
};
use des_ndl::{
    error::{Error, ErrorKind, Errors, ErrorsMut, RootError, RootResult},
    ir::{self, ConnectionEndpoint},
    Context,
};
use des_networks::ndl::{
    def::{self, Kardinality},
    error::{self, Result},
    transform, tree,
};
use std::{fs::File, path::Path, sync::Arc};

mod registry;
pub use self::registry::*;

// mod registry;
// pub use self::registry::*;

impl Sim<()> {
    /// Creates a NDL application with the inner application `()`.
    ///
    /// See [`Sim::ndl_with`] for more information.
    ///
    /// # Errors
    ///
    /// This function may return an error, if the provided NDL topology is
    /// erronous, or the software requirements cannot be fulfilled by the registry.
    pub fn ndl<L: Layer>(
        path: impl AsRef<Path>,
        registry: impl AsMut<Registry<L>>,
    ) -> RootResult<Self> {
        Sim::new(()).with_ndl(path, registry)
    }

    /// NEW
    pub fn ndl2<L: Layer>(
        path: impl AsRef<Path>,
        registry: impl AsMut<Registry<L>>,
    ) -> Result<Self> {
        Sim::new(()).with_ndl2(path, registry)
    }
}

impl<A> Sim<A> {
    /// Creates an NDL application from a topology description at `path`, with
    /// software defined by `registry` and an inner application `inner`.
    ///
    /// The NDL topology desciption found at `path` describes a module tree
    /// including a root module at the path `""`. Each node in this tree
    /// is derived from a NDL Module. The name of this module prototype
    /// is the symbol used in accessed to the registry. The NDL topology
    /// additionally includes gate and gate-chain definitions.
    ///
    /// The tree is initalized depth first. This means for each module:
    /// - First the gate of the current module are created
    /// - Then all children are created, including gates **and** connections
    /// - Then all connections are resolved, since connections statements may depend
    ///   on the existence of gates in child nodes
    ///
    /// The provided parameter `registry` is resposible for attaching software
    /// to the nodes defined by the topology description. Should the registry
    /// fail to provide software for a node, this function will fail.
    ///
    /// The inner application `inner` is equivalent the inner application
    /// object of a network simulation, which can be used to define custom
    /// actions at sim start / end.
    ///
    /// **NOTE** that the nodes will be created with a call to this function.
    ///
    /// # Errors
    ///
    /// Some Errors
    pub fn with_ndl<L: Layer>(
        mut self,
        path: impl AsRef<Path>,
        registry: impl AsMut<Registry<L>>,
    ) -> RootResult<Self> {
        self.nodes_from_ndl(path, registry)?;
        Ok(self)
    }

    /// NEW
    pub fn with_ndl2<L: Layer>(
        mut self,
        path: impl AsRef<Path>,
        registry: impl AsMut<Registry<L>>,
    ) -> Result<Self> {
        let f = File::open(path).map_err(|e| error::Error::Io(e.to_string()))?;
        let def = serde_yml::from_reader(f).map_err(|e| error::Error::Io(e.to_string()))?;
        self.nodes_from_ndl2(&def, registry)?;
        Ok(self)
    }

    /// Builds a NDL based application with onto an allready existing [`Sim`] object.
    ///
    /// See [`Sim::ndl_with`] for more infomation.
    ///
    /// # Errors
    ///
    /// This function will fail if either:
    /// a) some NDL error occures when parsing the NDL tree defined at `path`,
    /// b) or the registry fails to provide software for some NDL-defined module.
    #[allow(clippy::missing_panics_doc)]
    pub fn nodes_from_ndl<L: Layer>(
        &mut self,
        path: impl AsRef<Path>,
        mut registry: impl AsMut<Registry<L>>,
    ) -> RootResult<()> {
        let mut ctx = Context::load(path)?;
        let tree = ctx
            .entry
            .take()
            .expect("internal NDL error: entry point must be provided on Ok(Context)");

        let mut errors = Errors::new().as_mut();

        let scoped = ScopedSim::new(self, ObjectPath::default());
        let _ = scoped.ndl(tree, &mut errors, registry.as_mut());

        if errors.is_empty() {
            Ok(())
        } else {
            Err(RootError::new(errors.into_inner(), ctx.smap))
        }
    }

    /// NEW
    pub fn nodes_from_ndl2<L: Layer>(
        &mut self,
        def: &def::Def,
        mut registry: impl AsMut<Registry<L>>,
    ) -> Result<()> {
        let mut errors = Errors::new().as_mut();

        let parsed = transform(def)?;

        let scoped = ScopedSim::new(self, ObjectPath::default());
        let _ = scoped.ndl2(&parsed, &mut errors, registry.as_mut());

        if errors.is_empty() {
            Ok(())
        } else {
            println!("{:#?}", errors.into_inner());
            Err(error::Error::Other)
        }
    }

    fn raw_ndl<L: Layer>(
        &mut self,
        path: &ObjectPath,
        ty: &str,
        registry: &mut Registry<L>,
        errors: &mut ErrorsMut,
    ) -> ModuleRef {
        // Check dup
        assert!(
            self.modules().get(path).is_none(),
            "cannot crate module at {path}, allready exists"
        );

        // Check node path location
        let ctx = if let Some(parent) = path.parent() {
            // (a) Check that the parent exists
            let parent = self
                .get(&parent)
                .expect("cannot create module, parent missing in NDL build");

            ModuleContext::child_of(path.name(), parent)
        } else {
            ModuleContext::standalone(path.clone())
        };
        ctx.activate();

        if let Some(software) = registry.resolve(path, ty, &mut *self.stack) {
            ctx.upgrade_dummy(software);
        } else {
            errors.add(Error::new(
                ErrorKind::SymbolNotFound,
                format!("symbol '{ty}' at '{path}' could not be resolved by the registry"),
            ));
        }

        // TODO: deactivate module
        self.modules_mut().add(ctx.clone());
        ctx
    }
}

impl<'a, A> ScopedSim<'a, A> {
    fn ndl2<L: Layer>(
        mut self,
        node: &tree::Node,
        errors: &mut ErrorsMut,
        registry: &mut Registry<L>,
    ) -> ModuleRef {
        let symbol = node.typ.to_string();
        let scope = &self.scope;

        let ctx = self.base.raw_ndl(scope, &symbol, registry, errors);

        for gate in &node.gates {
            let _ = ctx.create_gate_cluster(&gate.ident, gate.kardinality.as_size());
        }

        for submodule in &node.submodules {
            match submodule.name.kardinality {
                Kardinality::Atom => {
                    let subscope = self.subscope(&submodule.name.ident);
                    subscope.ndl2(&submodule.typ, errors, registry);
                }
                Kardinality::Cluster(n) => {
                    for k in 0..n {
                        let ident = &submodule.name.ident;
                        let subscope = self.subscope(format!("{ident}[{k}]"));
                        subscope.ndl2(&submodule.typ, errors, registry);
                    }
                }
            }
        }

        fn access_gate(
            ctx: &ModuleContext,
            accessors: &[tree::ConnectionEndpointAccessor],
        ) -> Option<net::gate::GateRef> {
            assert!(accessors.len() > 0);
            let accessor = &accessors[0];
            if accessors.len() == 1 {
                // Gate access
                ctx.gate(&accessor.name, accessor.index.unwrap_or(0))
            } else {
                // Submodule access
                let child = ctx.child(&accessor.as_name()).expect("child");
                access_gate(&child.ctx, &accessors[1..])
            }
        }

        for connection in &node.connections {
            let from = access_gate(&ctx.ctx, &connection.peers[0].accessors).expect("gate");
            let to = access_gate(&ctx.ctx, &connection.peers[1].accessors).expect("gate");

            from.connect_dedup(
                to,
                connection
                    .link
                    .as_ref()
                    .map(|link| Channel::new(ChannelMetrics::from(link))),
            )
        }

        ctx
    }

    #[allow(clippy::needless_pass_by_value)]
    fn ndl<L: Layer>(
        mut self,
        ir: Arc<ir::Module>,
        errors: &mut ErrorsMut,
        registry: &mut Registry<L>,
    ) -> ModuleRef {
        let ty = ir.ident.raw.clone();
        let scope = &self.scope;

        let ctx = self.base.raw_ndl(scope, &ty, registry, errors);

        for gate in &ir.gates {
            let _ = ctx.create_gate_cluster(&gate.ident.raw, gate.cluster.as_size());
        }

        for submod in &ir.submodules {
            let sir = submod.typ.as_module_arc().expect(
                "invalid NDL tree: submodule typ referes does not refer to a module object",
            );

            match submod.cluster {
                ir::Cluster::Standalone => {
                    // Resue current struct better for stack
                    let subscope = self.subscope(&submod.ident.raw);
                    subscope.ndl(sir, errors, registry);
                }
                ir::Cluster::Clusted(n) => {
                    for k in 0..n {
                        let ident = &submod.ident.raw;
                        let subscope = self.subscope(format!("{ident}[{k}]"));
                        subscope.ndl(sir.clone(), errors, registry);
                    }
                }
            }
        }

        for con in &ir.connections {
            let from = match &con.lhs {
                ConnectionEndpoint::Local(gate, pos) => ctx.gate(&gate.raw, pos.as_index()),
                ConnectionEndpoint::Nonlocal(submod, pos, gate) => {
                    let c = ctx
                        .child(&format!("{}{}", submod.raw, pos))
                        .expect("invalid NDL tree: connection refer to child that does not exist");
                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .expect("invalid NDL tree: connections refer to gate(-cluster) not defined by NDL");

            let to = match &con.rhs {
                ConnectionEndpoint::Local(gate, pos) => ctx.gate(&gate.raw, pos.as_index()),
                ConnectionEndpoint::Nonlocal(submod, pos, gate) => {
                    let c = ctx
                        .child(&format!("{}{}", submod.raw, pos))
                        .expect("invalid NDL tree: connection refer to child that does not exist");
                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .expect("invalid NDL tree: connections refer to gate(-cluster) not defined by NDL");

            from.connect_dedup(
                to,
                if let Some(delay) = &con.delay {
                    let link = delay
                        .as_link_arc()
                        .expect("invalid NDL tree: link typ does not refer to a link object");
                    Some(Channel::new(ChannelMetrics::from(&*link)))
                } else {
                    None
                },
            );
        }

        ctx
    }
}

impl From<&tree::Link> for ChannelMetrics {
    #[allow(clippy::cast_sign_loss)]
    fn from(value: &tree::Link) -> Self {
        ChannelMetrics {
            bitrate: value.bitrate as usize,
            jitter: Duration::from_secs_f64(value.jitter),
            latency: Duration::from_secs_f64(value.latency),
            drop_behaviour: ChannelDropBehaviour::Queue(Some(
                value
                    .other
                    .get("queuesize")
                    .map_or(0, |v| v.parse().expect("number")) as usize,
            )),
        }
    }
}

impl From<&ir::Link> for ChannelMetrics {
    #[allow(clippy::cast_sign_loss)]
    fn from(value: &ir::Link) -> Self {
        ChannelMetrics {
            bitrate: value.bitrate as usize,
            jitter: Duration::from_secs_f64(value.jitter),
            latency: Duration::from_secs_f64(value.latency),
            drop_behaviour: ChannelDropBehaviour::Queue(Some(
                value
                    .fields
                    .get("queuesize")
                    .map_or(0, ir::Literal::as_integer_casted) as usize,
            )),
        }
    }
}
