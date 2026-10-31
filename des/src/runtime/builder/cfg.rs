use std::{fmt::Debug, sync::Arc};

use crate::{module::UnwindBehaviour, processing::ProcessingStack};

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
