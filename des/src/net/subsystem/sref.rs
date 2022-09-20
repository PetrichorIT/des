use crate::prelude::ObjectPath;

use super::{Subsystem, SubsystemContext};
use std::{
    cell::RefCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// A reference to a subsystem
#[derive(Clone)]
pub struct SubsystemRef {
    pub(crate) ctx: Arc<SubsystemContext>,
    handler: Arc<RefCell<dyn Subsystem>>,
    handler_ptr: *mut u8,
}

impl SubsystemRef {
    /// Creates the main subsystem
    #[allow(clippy::explicit_deref_methods)]
    pub fn main<T>(subsystem: T) -> Self
    where
        T: Subsystem,
    {
        let handler = Arc::new(RefCell::new(subsystem));
        let ptr: *mut T = handler.borrow_mut().deref_mut();
        let ptr = ptr.cast::<u8>();

        let ctx = Arc::new(SubsystemContext::new_with(ObjectPath::root_subsystem(
            "root".to_string(),
        )));

        Self {
            ctx,
            handler,
            handler_ptr: ptr,
        }
    }
}

impl Deref for SubsystemRef {
    type Target = SubsystemContext;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl Debug for SubsystemRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubsystemRef").finish()
    }
}
