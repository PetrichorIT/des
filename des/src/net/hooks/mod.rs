//! Module-specifc message preprocessors.

use super::module::ModuleContext;
use super::{message::Message, module::with_mod_ctx};
use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;

mod periodic;
pub use periodic::PeriodicHook;

mod routing;
pub use routing::{RoutingHook, RoutingHookOptions};

/// A module-specific message preprocessor.
pub trait Hook {
    /// The message preprocessor defined by this hook.
    ///
    /// This function takes the hooks internal state and an arriving
    /// message as input. It may return Ok(()) if the message was consumed
    /// by the hook or Err(msg) when the message should be further processed.
    fn handle_message(&mut self, msg: Message) -> Result<(), Message>;
}

///
/// Creates a new module-specific hook.
///
/// A hook acts as a message preprocessor which handles messages before
/// they reach the handle_message function. In this preprocessing
/// the message can be:
///
/// - consumed, leading to no activitation of handle_message
/// - changed, still leading to an activiation of handle_message (unless another hook consumes the message)
/// - remain untouched
///
/// Hooks are attached to the current module and only process message
/// bound for this moudle. Set a priority to define the order
/// in which the hooks are evaluated. Lower values equate to
/// a higher priority.
///
/// Hooks are usually created in the at_sim_start method of either the [Module]
/// or the [AsyncModule] trait. This is the case, because multiple calls of [create_module]
/// result in multiple distinct hooks being created. If a module is shutdown and restarted,
/// all hooks are removed at shutdown and can be created anew when at_sim_start is
/// called following the modules restart.
///
/// # Examples
///
/// ```rust
/// use des::prelude::*;
/// use des::net::hooks::PeriodicHook;
///
/// const INITIAL_COUNT: usize = 42;
///
/// #[NdlModule]
/// struct Clock;
///
/// impl Module for Clock {
///     fn new() -> Self {
///         Clock
///     }
///
///     fn at_sim_start(&mut self, _stage: usize) {
///         create_hook(
///             PeriodicHook::new(|counter| send(Message::new().content(*counter).build(), "out"),
///                 Duration::from_secs(1),
///                 INITIAL_COUNT
///             ),
///             100
///         )
///     }
/// }
/// ```
pub fn create_hook(hook: impl Hook + 'static, priority: usize) {
    with_mod_ctx(|ctx| ctx.create_hook(hook, priority))
}

impl ModuleContext {
    /// Refer to [create_hook].
    pub fn create_hook(&self, hook: impl Hook + 'static, priority: usize) {
        let entry = HookEntry {
            hook: Box::new(hook),
            priority,
        };

        let mut hooks = self.hooks.borrow_mut();
        match hooks.binary_search(&entry) {
            Ok(at) => hooks.insert(at, entry),
            Err(at) => hooks.insert(at, entry),
        };
    }
}

// # INTERNALS

pub(crate) struct HookEntry {
    pub(crate) hook: Box<dyn Hook>,
    pub(crate) priority: usize,
}

impl Deref for HookEntry {
    type Target = Box<dyn Hook>;
    fn deref(&self) -> &Self::Target {
        &self.hook
    }
}

impl DerefMut for HookEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.hook
    }
}

impl PartialEq for HookEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for HookEntry {}

impl PartialOrd for HookEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HookEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl Debug for HookEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookEntry").finish()
    }
}
