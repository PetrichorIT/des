//! Module-level gneric hooks.

use super::Message;
use std::any::Any;

/// A hook into a modules packet managment
pub trait Hook {
    /// REturns the current state
    fn state(&self) -> &dyn Any;
    /// handles a message
    fn handle_message(&mut self, msg: Message) -> Result<(), Message>;
}

mod periodic;
pub use periodic::PeriodicHook;

mod routing;
pub use routing::{RoutingHook, RoutingHookOptions};
