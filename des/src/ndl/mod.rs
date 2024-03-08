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
    net::{
        channel::ChannelDropBehaviour, module::ModuleContext, processing::ProcessingElements,
        ScopedSim, Sim,
    },
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
    /// NDL
    pub fn ndl(path: impl AsRef<Path>, registry: Registry) -> RootResult<Self> {
        Self::ndl_with(path, registry, ())
    }
}

impl<A> Sim<A> {
    /// NDL
    pub fn ndl_with(path: impl AsRef<Path>, registry: Registry, inner: A) -> RootResult<Self> {
        let mut ctx = Context::load(path)?;
        let tree = ctx.entry.take().unwrap();

        // Build network
        let mut this = Sim::new(inner);
        let mut errors = Errors::new().as_mut();

        let scoped = ScopedSim::new(&mut this, ObjectPath::new());
        let _ = scoped.ndl(tree, &mut errors, &registry);

        if errors.is_empty() {
            Ok(this)
        } else {
            Err(RootError::new(errors.into_inner(), ctx.smap))
        }
    }

    fn raw_ndl(&mut self, path: ObjectPath, pe: ProcessingElements) -> ModuleRef {
        // Check dup
        if self.modules.get(&path).is_some() {
            panic!("cannot crate module at {path}, allready exists");
        }

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
        ctx.upgrade_dummy(pe);

        // TODO: deactivate module
        self.modules.insert(path, ctx.clone());
        ctx
    }
}

impl<'a, A> ScopedSim<'a, A> {
    fn ndl(
        mut self,
        ir: Arc<ir::Module>,
        errors: &mut ErrorsMut,
        registry: &Registry,
    ) -> Option<ModuleRef> {
        let ty = ir.ident.raw.clone();
        let scope = &self.scope;
        let Some(block) = registry.lookup(scope, &ty) else {
            errors.add(Error::new(
                ErrorKind::SymbolNotFound,
                format!("symbol '{ty}' at '{scope}' could not be resolved by the registry"),
            ));
            return None;
        };

        let ctx = self.base.raw_ndl(scope.clone(), block);

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
                    Some(Channel::new(
                        ctx.path().appended_channel("ch"),
                        ChannelMetrics::from(&*link),
                    ))
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
