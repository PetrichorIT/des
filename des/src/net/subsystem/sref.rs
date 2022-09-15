use super::{Subsystem, SubsystemContext};
use std::{cell::RefCell, fmt::Debug, sync::Arc};

/// A reference to a subsystem
pub struct SubsystemRef {
    pub(crate) ctx: Arc<SubsystemContext>,
    handler: Arc<RefCell<dyn Subsystem>>,
    handler_ptr: *mut u8,
}

impl Debug for SubsystemRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubsystemRef").finish()
    }
}
