//! Integration of the Network-Description-Language (NDL).
//!
//! # What is NDL ?
//!
//! NDL is a decriptory language for defining network topologies.
//! Refer to [`des_ndl`] for more information.
//!
//! # How to use it ?
//!
//! This submodule provides an [`NdlApplication`] that can be passed to
//! a network runtime, to build a network based on a given topology.
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
//! struct ModuleA;
//! /* ... */
//!
//! struct ModuleB;
//! /* ... */
//!
//! # impl Module for ModuleA { fn new() -> Self { Self }}
//! # impl Module for ModuleB { fn new() -> Self { Self }}
//! fn main() {
//!     # return;
//!     let app = match NdlApplication::new("path/to/ndl.ndl", registry![ModuleA, ModuleB]) {
//!         Ok(v) => v,
//!         Err(e) => {
//!             println!("{e}");
//!             return;
//!         },
//!     };
//!     let rt = Builder::new().build(NetworkApplication::new(app));
//!     let _ = rt.run();
//! }
//! ```

use crate::{
    net::{channel::ChannelDropBehaviour, module::ModuleContext},
    prelude::{
        Channel, ChannelMetrics, EventLifecycle, ModuleRef, NetworkApplication,
        ObjectPath,
    },
    time::Duration,
};
use des_ndl::{
    ast::Spanned,
    error::{Error, ErrorKind, Errors, RootError, RootResult},
    ir::{self, ConnectionEndpoint},
    Context,
};
use std::{path::Path, sync::Arc};

mod registry;
pub use self::registry::*;

/// An application that creates a network-like
/// simulation from a Ndl-Topology description.
///
/// Use this type to manage loading of Ndl-Assets and parameter files.
/// Upon creation this type can be passed to a [`NetworkApplication`]
/// to instanitate a network simulation. When the simulation is executed
/// this type holds a reference to the network modules itself, which
/// can then be extraced after from a [`RuntimeResult`](crate::runtime::RuntimeResult).
#[derive(Debug)]
pub struct NdlApplication {
    handle: Option<ModuleRef>,
    tree: Arc<ir::Module>,
    registry: Registry,
}

impl NdlApplication {
    /// Turns the NDL builder into a `NetworkApplication`, ready to be
    /// used by a `Runtime`.
    #[must_use]
    pub fn into_app(self) -> NetworkApplication<NdlApplication> {
        NetworkApplication::new(self)
    }

    /// Returns a handle to the simulated network.
    ///
    /// This function returns None, if the network was not yet created.
    /// After initalizing the [`Runtime`](crate::runtime::Runtime)
    /// there should allways be a network.
    #[must_use]
    pub fn network(&self) -> Option<&ModuleRef> {
        self.handle.as_ref()
    }

    /// Returns a handle to the topology, described by the Ndl-Assets.
    #[must_use]
    pub fn topology(&self) -> Arc<ir::Module> {
        self.tree.clone()
    }

    /// Creates a new `NdlApplication` using the provided path as
    /// root for the Ndl-Assets and the registry of types for binding.
    ///
    /// # Errors
    ///
    /// This function may fail if either the assets are in any way invalid,
    /// or the registry does not provide a link to a type, required by the
    /// assets.
    ///
    #[allow(clippy::missing_panics_doc)]
    pub fn new(path: impl AsRef<Path>, registry: Registry) -> RootResult<NdlApplication> {
        let mut ctx = Context::load(path)?;
        let tree = ctx.entry.take().unwrap();
        let symbols = ir::Module::all_modules(tree.clone());
        let mut missing = Vec::new();
        for symbol in symbols {
            if registry.get(&symbol.ident.raw).is_none() {
                missing.push((symbol.ident.raw.clone(), symbol.ast.span()));
            }
        }

        if missing.is_empty() {
            Ok(NdlApplication {
                tree,
                registry,
                handle: None,
            })
        } else {
            let mut errors = Errors::new().as_mut();
            for (sym, span) in missing {
                errors.add(
                    Error::new(
                        ErrorKind::SymbolNotFound,
                        format!("Symbol '{sym}' is required, but was not found in registry"),
                    )
                    .spanned(span),
                );
            }
            Err(RootError::new(errors.into_inner(), ctx.smap))
        }
    }
}

impl EventLifecycle<NetworkApplication<Self>> for NdlApplication {
    fn at_sim_start(rt: &mut crate::prelude::Runtime<NetworkApplication<Self>>) {
        rt.app.inner.handle = Some(Self::build_at(
            rt,
            rt.app.inner.tree.clone(),
            &ObjectPath::new(),
            None,
        ));
    }
}

impl NdlApplication {
    #[allow(clippy::needless_pass_by_value)]
    fn build_at(
        rt: &mut crate::prelude::Runtime<NetworkApplication<Self>>,
        ir: Arc<ir::Module>,
        path: &ObjectPath,
        parent: Option<ModuleRef>,
    ) -> ModuleRef {
        let ident = path.name();
        let ty = &ir.ident.raw;

        let ctx = if let Some(parent) = parent {
            ModuleContext::child_of(ident, parent)
        } else {
            ModuleContext::standalone(ObjectPath::new())
        };

        for gate in &ir.gates {
            let _ = ctx.create_gate_cluster(
                &gate.ident.raw,
                gate.cluster.as_size(),
                // gate.service_typ.into(),
            );
        }

        for submod in &ir.submodules {
            let sty = submod.typ.as_module_arc().unwrap();

            match submod.cluster {
                ir::Cluster::Standalone => {
                    let new_path = path.appended(&submod.ident.raw);
                    Self::build_at(rt, sty, &new_path, Some(ctx.clone()));
                }
                ir::Cluster::Clusted(n) => {
                    for k in 0..n {
                        let new_path = path.appended(&format!("{}[{}]", submod.ident.raw, k));
                        Self::build_at(rt, sty.clone(), &new_path, Some(ctx.clone()));
                    }
                }
            };

            // ctx.add_child(&submod.ident.raw, sub);
        }

        for con in &ir.connections {
            let from = match &con.from {
                ConnectionEndpoint::Local(gate, pos) => ctx.gate(&gate.raw, pos.as_index()),
                ConnectionEndpoint::Nonlocal(submod, pos, gate) => {
                    let c = ctx.child(&format!("{}{}", submod.raw, pos)).unwrap();
                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .unwrap();

            let to = match &con.to {
                ConnectionEndpoint::Local(gate, pos) => ctx.gate(&gate.raw, pos.as_index()),
                ConnectionEndpoint::Nonlocal(submod, pos, gate) => {
                    let c = ctx.child(&format!("{}{}", submod.raw, pos)).unwrap();
                    c.gate(&gate.0.raw, gate.1.as_index())
                }
            }
            .unwrap();



            from.connect_dedup(to,  if let Some(delay) = &con.delay {
                let link = delay.as_link_arc().unwrap();
                Some(Channel::new(
                    ctx.path().appended_channel("ch"),
                    ChannelMetrics::from(&*link),
                ))

            } else {
                None
            });
           
        }

        ctx.activate();

        let f = rt.app.inner.registry.get(ty).unwrap();
        let state = f();
        ctx.upgrade_dummy(state);

        rt.app.register_module(ctx.clone());

        ctx
    }
}

// impl From<ir::GateServiceType> for GateServiceType {
//     fn from(value: ir::GateServiceType) -> Self {
//         match value {
//             ir::GateServiceType::Input => GateServiceType::Input,
//             ir::GateServiceType::Output => GateServiceType::Output,
//             ir::GateServiceType::None => GateServiceType::Undefined,
//         }
//     }
// }

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
