use std::{fmt::Debug, sync::Arc};

use crate::{Failure, Sim, module::UnwindBehaviour, processing::ProcessingStack};

#[derive(Clone)]
pub(crate) struct SimConfiguration {
    pub stack: Arc<dyn Fn() -> ProcessingStack>,
    pub default_unwind_behavior: UnwindBehaviour,
}

impl Debug for SimConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimConfiguration").finish()
    }
}

/// A trait for sim events
pub trait SimLifecycle: Sized {
    /// See [`Application::at_sim_start`]
    ///
    /// # Errors
    ///
    /// Errors that may occur during the simulation start event.
    ///
    /// [`Application::at_sim_start`]: crate::runtime::Application::at_sim_start
    fn at_sim_start(_rt: &mut Sim<Self>) -> Result<(), Failure> {
        Ok(())
    }
    /// See [`Application::at_sim_end`]
    ///
    /// # Errors
    ///
    /// Errors that may occur during the simulation end event.
    ///
    /// [`Application::at_sim_end`]: crate::runtime::Application::at_sim_end
    fn at_sim_end(_rt: &mut Sim<Self>) -> Result<(), Failure> {
        Ok(())
    }
}

impl SimLifecycle for () {}
