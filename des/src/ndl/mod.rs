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
    net::{channel::ChannelDropBehaviour, module::ModuleContext, ScopedSim, Sim},
    prelude::{Channel, ChannelMetrics, ModuleRef, ObjectPath},
    time::Duration,
};
use des_ndl::{
    error::{Error, ErrorKind, Errors, ErrorsMut, RootError, RootResult},
    ir::{self, ConnectionEndpoint},
    Context,
};
use std::{path::Path, sync::Arc};

mod registry;
pub use self::registry::*;

impl Sim<()> {
    /// Creates a NDL application with the inner application `()`.
    ///
    /// See [`Sim::ndl_with`] for more information.
    ///
    /// # Errors
    ///
    /// This function may return an error, if the provided NDL topology is
    /// erronous, or the software requirements cannot be fulfilled by the registry.
    pub fn ndl(path: impl AsRef<Path>, registry: impl AsRef<Registry>) -> RootResult<Self> {
        Self::ndl_with(path, registry, ())
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
    pub fn ndl_with(
        path: impl AsRef<Path>,
        registry: impl AsRef<Registry>,
        inner: A,
    ) -> RootResult<Self> {
        let mut this = Sim::new(inner);
        this.build_ndl(path, registry)?;
        Ok(this)
    }

    /// Builds a NDL based application with onto an allready existing [`Sim`] object.
    ///
    /// See [`Sim::ndl_with`] for more infomation.
    pub fn build_ndl(
        &mut self,
        path: impl AsRef<Path>,
        registry: impl AsRef<Registry>,
    ) -> RootResult<()> {
        let mut ctx = Context::load(path)?;
        let Some(tree) = ctx.entry.take() else {
            return Err(RootError::single(
                Error::new(ErrorKind::MissingEntryPoint, ""),
                ctx.smap,
            ));
        };

        let mut errors = Errors::new().as_mut();

        let scoped = ScopedSim::new(self, ObjectPath::new());
        let _ = scoped.ndl(tree, &mut errors, registry.as_ref());

        if errors.is_empty() {
            Ok(())
        } else {
            Err(RootError::new(errors.into_inner(), ctx.smap))
        }
    }

    fn raw_ndl(&mut self, path: &ObjectPath, ty: &str, registry: &Registry) -> Option<ModuleRef> {
        // Check dup
        assert!(
            self.modules.get(&path).is_none(),
            "cannot crate module at {path}, allready exists"
        );

        // Check node path location
        let ctx = if let Some(parent) = path.parent() {
            // (a) Check that the parent exists
            let Some(parent) = self.get(&parent) else {
                panic!("cannot create module at {path}, since parent module at {parent} is required, but not existent");
            };

            ModuleContext::child_of(path.name(), parent)
        } else {
            ModuleContext::standalone(path.clone())
        };
        ctx.activate();
        ctx.upgrade_dummy(registry.lookup(&path, ty)?);

        // TODO: deactivate module
        self.modules.add(ctx.clone());
        Some(ctx)
    }
}

impl<'a, A> ScopedSim<'a, A> {
    #[allow(clippy::needless_pass_by_value)]
    fn ndl(
        mut self,
        ir: Arc<ir::Module>,
        errors: &mut ErrorsMut,
        registry: &Registry,
    ) -> Option<ModuleRef> {
        let ty = ir.ident.raw.clone();
        let scope = &self.scope;

        let Some(ctx) = self.base.raw_ndl(scope, &ty, registry) else {
            errors.add(Error::new(
                ErrorKind::SymbolNotFound,
                format!("symbol '{ty}' at '{scope}' could not be resolved by the registry"),
            ));
            return None;
        };

        for gate in &ir.gates {
            let _ = ctx.create_gate_cluster(&gate.ident.raw, gate.cluster.as_size());
        }

        for submod in &ir.submodules {
            let sir = submod.typ.as_module_arc().unwrap();

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
                    let c = ctx.child(&format!("{}{}", submod.raw, pos)).unwrap();
                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .unwrap();

            let to = match &con.rhs {
                ConnectionEndpoint::Local(gate, pos) => ctx.gate(&gate.raw, pos.as_index()),
                ConnectionEndpoint::Nonlocal(submod, pos, gate) => {
                    let c = ctx.child(&format!("{}{}", submod.raw, pos)).unwrap();
                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .unwrap();

            from.connect_dedup(
                to,
                if let Some(delay) = &con.delay {
                    let link = delay.as_link_arc().unwrap();
                    Some(Channel::new(ChannelMetrics::from(&*link)))
                } else {
                    None
                },
            );
        }

        Some(ctx)
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
