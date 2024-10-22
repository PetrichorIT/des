use super::globals;
use crate::net::module::current;
use std::io;

pub use des_net_utils::par::Par;
pub use des_net_utils::par::ParError;

/// Retrieves a simulation parameter attached to the current node.
///
/// > *This function requires a node-context within the simulation*
///
/// The retrieved [`Par`] object points to a potentially existent parameter
/// assigned to the current node. If non-existent the `Par` object can be
/// used to set a the parametern for the first time. Parameters are stored
/// as strings internally.
///
/// # Examples
///
/// ```
/// # use des::prelude::*;
/// struct MyModule;
/// impl Module for MyModule {
///     fn at_sim_start(&mut self, stage: usize) {
///         let hostname = par("hostname");
///         assert!(hostname.is_some());
///     }
/// }
///
/// let mut sim = Sim::new(());
/// sim.node("alice", MyModule);
/// ```
#[must_use]
pub fn par(key: impl AsRef<str>) -> Par {
    Par::new(
        globals().parameters.clone(),
        key.as_ref(),
        current().path().as_str(),
    )
}

/// Retrieves a simulation parameter from some node in the simulation.
///
/// > *This function should only be called while the simulation is active*
///
/// The retrieved [`Par`] object behaves, as if retrived by [`par`] on the given
/// node. See [`par`] for more information.
#[must_use]
pub fn par_for(key: impl AsRef<str>, module: impl AsRef<str>) -> Par {
    Par::new(globals().parameters.clone(), key.as_ref(), module.as_ref())
}

/// Exports the current simulation parameter tree to some output device.
///
/// The output will be encoded as a key-value list where each key-value
/// pair is one line, sperated by a '='.
///
/// # Errors
///
/// This function may fail if write operations to the output
/// fails.
pub fn par_export(mut into: impl io::Write) -> io::Result<()> {
    globals().parameters.export(&mut into)
}
