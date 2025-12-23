use std::{fmt::Debug, sync::Arc};

use crate::{
    net::{Sim, module::UnwindBehaviour, processing::ProcessingStack},
    prelude::{Runtime, RuntimeError},
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
    fn at_sim_start(_rt: &mut Runtime<Sim<Self>>) -> Result<(), RuntimeError> {
        Ok(())
    }
    /// See [`Application::at_sim_end`]
    fn at_sim_end(_rt: &mut Runtime<Sim<Self>>) -> Result<(), RuntimeError> {
        Ok(())
    }
}

impl SimLifecycle for () {}
