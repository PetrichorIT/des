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
//! # use des::net::ndl::*;
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
use des_net_utils::ndl::{
    error::{self, ErrorKind, Result},
    transform,
    tree::{self, Node},
};
use std::{
    fs::{self, File},
    path::Path,
};

pub use des_net_utils::ndl::def::*;

mod registry;
pub use self::registry::*;

use super::ModuleBlock;

/// Inject modules described using the Node Description Language (NDL).
///
/// A NDL topology describes a module tree, that can be dynamically created
/// using modules provided in a [`Registry`]. This module tree can either be
/// attached at a specific location in the simulation module tree using
/// [`Sim::node`] with [`Ndl`] as the provided module block, or as a global
/// tree using constructors like [`Sim::ndl`].
///
/// The tree is initalized depth first. This means for each module:
/// - First the gate of the current module are created
/// - Then all children are created, including gates **and** connections
/// - Then all connections are resolved, since connections statements may depend
///   on the existence of gates in child nodes
///
/// To initalize a node, the parameter `registry` is used to provide
/// an implementation of the [`Module`] trait. Should the registry
/// fail to provide an implementation, the node creation will fail.
#[derive(Debug)]
pub struct Ndl<'a, L: Layer> {
    registry: &'a mut Registry<L>,
    node: Node,
}

impl<'a, L: Layer> Ndl<'a, L> {
    /// Loads a NDL topology description from a raw `Def` and a provided registry.
    ///
    /// # Errors
    ///
    /// This function may return an error, if the provided NDL topology is
    /// invalid or if the registry fails to provide an implementation for a module.
    pub fn new(registry: &'a mut Registry<L>, def: &Def) -> Result<Self> {
        Ok(Self {
            registry,
            node: transform(def)?,
        })
    }

    /// Loads a NDL topology description from a file and a provided registry.
    ///
    /// # Errors
    ///
    /// This function may return an error, if the provided NDL topology is
    /// invalid or if the registry fails to provide an implementation for a module.
    pub fn from_str(registry: &'a mut Registry<L>, str: &str) -> Result<Self> {
        let def = serde_yml::from_str(str).map_err(|e| ErrorKind::Io(e.to_string()))?;
        Self::new(registry, &def)
    }

    /// Loads a NDL topology description from a file and a provided registry.
    ///
    /// # Errors
    ///
    /// This function may return an error, if the provided NDL topology is
    /// invalid or if the registry fails to provide an implementation for a module.
    pub fn from_file(registry: &'a mut Registry<L>, path: impl AsRef<Path>) -> Result<Self> {
        let str = fs::read_to_string(path).map_err(|e| ErrorKind::Io(e.to_string()))?;
        Self::from_str(registry, &str)
    }
}

impl<L: Layer> ModuleBlock for Ndl<'_, L> {
    type Ret = Result<ModuleRef>;
    fn build<A>(self, sim: ScopedSim<'_, A>) -> Self::Ret {
        sim.ndl(&self.node, self.registry)
    }
}

//

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
    ) -> Result<Self> {
        Sim::new(()).with_ndl(path, registry)
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
    ) -> Result<Self> {
        let f = File::open(path).map_err(|e| error::ErrorKind::Io(e.to_string()))?;
        let def = serde_yml::from_reader(f).map_err(|e| error::ErrorKind::Io(e.to_string()))?;
        self.nodes_from_ndl(&def, registry)?;
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
    pub fn nodes_from_ndl<L: Layer>(
        &mut self,
        def: &Def,
        mut registry: impl AsMut<Registry<L>>,
    ) -> Result<()> {
        let parsed = transform(def)?;

        let scoped = ScopedSim::new(self, ObjectPath::default());
        let _ = scoped.ndl(&parsed, registry.as_mut())?;

        Ok(())
    }

    fn raw_ndl<L: Layer>(
        &mut self,
        path: &ObjectPath,
        ty: &str,
        registry: &mut Registry<L>,
    ) -> Result<ModuleRef> {
        // Check dup
        assert!(
            self.modules().get(path).is_none(),
            "cannot crate module at {path}, already exists"
        );

        // Check node path location
        let ctx = if let Some(parent) = path.nonzero_parent() {
            // (a) Check that the parent exists
            let parent = self
                .get(&parent)
                .expect("cannot create module, parent missing in NDL build");

            ModuleContext::child_of(path.name(), parent)
        } else if let Some(zero_parent) = self.get(&ObjectPath::from("")) {
            ModuleContext::child_of(path.name(), zero_parent)
        } else {
            ModuleContext::standalone(path.clone())
        };

        ctx.activate();
        let path_parts = ctx.path.as_str().split('.').collect::<Vec<_>>();
        for cfg in &self.cfgs {
            cfg.capture_for(&path_parts, &mut ctx.props.write());
        }

        let software = registry.resolve(path, ty, &mut *self.stack).ok_or(
            error::ErrorKind::MissingRegistrySymbol(path.to_string(), ty.to_string()),
        )?;
        ctx.upgrade_dummy(software);

        self.globals()
            .modules
            .lock()
            .expect("failed to lock globals")
            .push(ctx.clone());

        // TODO: deactivate module
        self.modules_mut().add(ctx.clone());
        Ok(ctx)
    }
}

impl<A> ScopedSim<'_, A> {
    fn ndl<L: Layer>(mut self, node: &tree::Node, registry: &mut Registry<L>) -> Result<ModuleRef> {
        let symbol = node.typ.to_string();
        let scope = &self.scope;

        let ctx = self.base.raw_ndl(scope, &symbol, registry)?;

        for gate in &node.gates {
            let _ = ctx.create_gate_cluster(&gate.ident, gate.kardinality.as_size());
        }

        for submodule in &node.submodules {
            match submodule.name.kardinality {
                Kardinality::Atom => {
                    let subscope = self.subscope(&submodule.name.ident);
                    subscope.ndl(&submodule.typ, registry)?;
                }
                Kardinality::Cluster(n) => {
                    for k in 0..n {
                        let ident = &submodule.name.ident;
                        let subscope = self.subscope(format!("{ident}[{k}]"));
                        subscope.ndl(&submodule.typ, registry)?;
                    }
                }
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
            );
        }

        Ok(ctx)
    }
}

fn access_gate(
    ctx: &ModuleContext,
    accessors: &[tree::ConnectionEndpointAccessor],
) -> Option<net::gate::GateRef> {
    assert!(!accessors.is_empty(), "accessors must be non-empty");
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
