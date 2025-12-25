use std::{fmt::Debug, sync::Arc};

use crate::{
    net::{Error, Sim, module::UnwindBehaviour, processing::ProcessingStack},
    prelude::Runtime,
};

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
    /// [`Application::at_sim_start`]: crate::runtime::Application::at_sim_start
    fn at_sim_start(_rt: &mut Runtime<Sim<Self>>) -> Result<(), Error> {
        Ok(())
    }
    /// See [`Application::at_sim_end`]
    ///
    /// [`Application::at_sim_end`]: crate::runtime::Application::at_sim_end
    fn at_sim_end(_rt: &mut Runtime<Sim<Self>>) -> Result<(), Error> {
        Ok(())
    }
}

impl SimLifecycle for () {}
