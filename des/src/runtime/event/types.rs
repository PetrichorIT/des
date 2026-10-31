use crate::{Failure, Sim};

/// A trait for sim events
pub trait SimLifecycle: Sized {
    /// Startup function
    ///
    /// # Errors
    ///
    /// Errors that may occur during the simulation start event.
    ///
    fn at_sim_start(_rt: &mut Sim<Self>) -> Result<(), Failure> {
        Ok(())
    }
    /// Shutdown function
    ///
    /// # Errors
    ///
    /// Errors that may occur during the simulation end event.
    ///
    fn at_sim_end(_rt: &mut Sim<Self>) -> Result<(), Failure> {
        Ok(())
    }
}

impl SimLifecycle for () {}
