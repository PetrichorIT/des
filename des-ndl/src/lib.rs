#![warn(clippy::pedantic)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    unreachable_pub,
    clippy::dbg_macro
)]
//! Integration of the Network-Description-Language (NDL).
//!
//! # What is NDL ?
//!
//! NDL is a decriptory language for defining network topologies.
//!
//! # How to use it ?
//!
//! This submodule provides an [`Sim::ndl`] that can create a simulation
//! that builsd a network based on a given topology.
//! Users can create such an application by providing the path to the
//! root file of the NDL description, and by providing a registry of modules.
//! This registry is used to link names of network nodes in NDL to associated
//! structs that implmenent [`Module`].
//! By proving both parameters, the application will load the topology and check
//! whether the network can be build. If not an descriptive error will be returned.
//!
//! ```
//! # use des::prelude::*;
//! # use des_ndl::*;
//! # use des_ndl::registry;
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
//!     let _ = app.build().run();
//! }
//! ```

use des::{
    Sim, SimBuilder,
    channel::ChannelDropBehaviour,
    gate::{GateRef, IntoGate},
    module::ModuleContext,
    prelude::{DatarateChannel, DatarateChannelMetrics, Module, ModuleRef, ObjectPath, Spawner},
    runtime::IntoModuleTree,
    time::Duration,
};
use std::{
    fs::{self, File},
    path::Path,
};

pub mod lang;
mod registry;

#[macro_use]
mod macros;

#[cfg(test)]
mod tests;

use crate::lang::error::{ErrorKind, Result};

pub use self::registry::*;

/// Inject modules described using the Node Description Language (NDL).
///
/// A NDL topology describes a module tree, that can be dynamically created
/// using modules provided in a [`Registry`]. This module tree can either be
/// attached at a specific location in the simulation module tree using
/// [`SimBuilder::node`] with [`Ndl`] as the provided module block, or as a global
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
    node: lang::tree::Node,
}

impl<'a, L: Layer> Ndl<'a, L> {
    /// Loads a NDL topology description from a raw `Def` and a provided registry.
    ///
    /// # Errors
    ///
    /// This function may return an error, if the provided NDL topology is
    /// invalid or if the registry fails to provide an implementation for a module.
    pub fn new(registry: &'a mut Registry<L>, def: &lang::def::Def) -> Result<Self> {
        Ok(Self {
            registry,
            node: lang::transform(def)?,
        })
    }

    /// Loads a NDL topology description from a file and a provided registry.
    ///
    /// # Errors
    ///
    /// This function may return an error, if the provided NDL topology is
    /// invalid or if the registry fails to provide an implementation for a module.
    pub fn from_str(registry: &'a mut Registry<L>, str: &str) -> Result<Self> {
        let def = serde_norway::from_str(str).map_err(|e| ErrorKind::Io(e.to_string()))?;
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

impl<L: Layer> IntoModuleTree for Ndl<'_, L> {
    type Ret = Result<ModuleRef>;

    fn build<A>(self, spawner: Spawner<'_, A>) -> Self::Ret {
        build_tree(&self.node, self.registry, spawner)
    }
}

fn build_tree<A, L: Layer>(
    node: &lang::tree::Node,
    registry: &mut Registry<L>,
    mut spawner: Spawner<'_, A>,
) -> Result<ModuleRef> {
    let symbol = node.typ.to_string();
    let scope = spawner.scope().clone();

    let ctx = spawn_raw_node(&scope, &symbol, registry, &mut spawner)?;
    for gate in &node.gates {
        if let Some(size) = gate.kardinality.as_size() {
            for pos in 0..size {
                let _ = ctx.create_gate(&gate.ident, pos);
            }
        } else {
            let _ = ctx.create_gate_cluster(&gate.ident);
        }
    }

    for submodule in &node.submodules {
        match submodule.name.kardinality {
            lang::def::Kardinality::Atom => {
                let subscope = spawner.subscope(&submodule.name.ident);
                build_tree(&submodule.typ, registry, subscope)?;
            }
            lang::def::Kardinality::Cluster(n) => {
                for k in 0..n {
                    let ident = &submodule.name.ident;
                    let subscope = spawner.subscope(format!("{ident}[{k}]"));
                    build_tree(&submodule.typ, registry, subscope)?;
                }
            }
            lang::def::Kardinality::ClusterUnsized => {
                panic!("unsized clusters are not allowed for submodules")
            }
        }
    }

    for connection in &node.connections {
        let from = access_gate(&ctx, &connection.peers[0].accessors).expect("gate");
        let to = access_gate(&ctx, &connection.peers[1].accessors).expect("gate");

        from.connect_with(
            to,
            connection
                .link
                .as_ref()
                .map(|link| DatarateChannel::new(DatarateChannelMetrics::from(link))),
        );
    }

    Ok(ctx)
}

fn spawn_raw_node<A, L: Layer>(
    path: &ObjectPath,
    ty: &str,
    registry: &mut Registry<L>,
    spawner: &mut Spawner<'_, A>,
) -> Result<ModuleRef> {
    // use the creation fn, but bypass its lack of error handling
    let mut result = Ok(());
    let module = spawner.root_with_context(|| {
        match registry
            .resolve(path, ty)
            .ok_or(lang::error::ErrorKind::MissingRegistrySymbol(
                path.to_string(),
                ty.to_string(),
            )) {
            Ok(software) => software,
            Err(e) => {
                result = Err(e.into());
                Box::new(Dummy)
            }
        }
    });

    result.map(|()| module)
}

struct Dummy;
impl Module for Dummy {}

/// An extension trait for [`Sim`] that provides NDL-related functionality.
pub trait SimExt {
    /// Loads a NDL tree at the given path as the root of the tree.
    ///
    /// # Errors
    ///
    /// May return an NDL language error.
    fn ndl<L: Layer>(
        path: impl AsRef<Path>,
        registry: impl AsMut<Registry<L>>,
    ) -> Result<SimBuilder<()>>;
}

impl SimExt for Sim<()> {
    fn ndl<L: Layer>(
        path: impl AsRef<Path>,
        mut registry: impl AsMut<Registry<L>>,
    ) -> Result<SimBuilder<()>> {
        let f = File::open(path).map_err(|e| lang::error::ErrorKind::Io(e.to_string()))?;
        let def =
            serde_norway::from_reader(f).map_err(|e| lang::error::ErrorKind::Io(e.to_string()))?;
        let ndl = Ndl::new(registry.as_mut(), &def)?;
        let mut sim = Sim::new(());
        sim.node("", ndl)?;
        Ok(sim)
    }
}

fn access_gate(
    ctx: &ModuleContext,
    accessors: &[lang::tree::ConnectionEndpointAccessor],
) -> Option<GateRef> {
    assert!(!accessors.is_empty(), "accessors must be non-empty");
    let accessor = &accessors[0];
    if accessors.len() == 1 {
        // Gate access
        ctx.gate((&accessor.name[..], accessor.index.unwrap_or(0)))
    } else {
        // Submodule access
        let child = ctx.child(&accessor.as_name()).expect("child");
        access_gate(&child, &accessors[1..])
    }
}

impl From<&lang::tree::Link> for DatarateChannelMetrics {
    #[allow(clippy::cast_sign_loss)]
    fn from(value: &lang::tree::Link) -> Self {
        DatarateChannelMetrics {
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
