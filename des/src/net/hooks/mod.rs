//! Module-specifc message preprocessors.

use super::module::ModuleContext;
use super::{message::Message, module::with_mod_ctx};
use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicUsize, Ordering};

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

/// A handle to a hook that allows the lifetime managment
/// of hooks.
#[derive(PartialEq, Eq, Hash)]
pub struct HookHandle {
    id: usize,

    #[cfg(debug_assertions)]
    hook_info: String,
}

impl Debug for HookHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(debug_assertions)]
        return write!(f, "Hook #{} {{ {} }}", self.id, self.hook_info);

        #[cfg(not(debug_assertions))]
        return write!(f, "Hook #{}", self.id);
    }
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
///         );
///     }
/// }
/// ```
pub fn create_hook(hook: impl Hook + 'static, priority: usize) -> HookHandle {
    with_mod_ctx(|ctx| ctx.create_hook(hook, priority))
}

///
/// Destroys the hook described by this handle.
///
/// # Panics
///
/// This function panics, if the hooks was not defined in the context of this module.
/// Also panics if executed outside of a module context.
///
pub fn destroy_hook(handle: HookHandle) {
    with_mod_ctx(|ctx| ctx.destroy_hook(handle))
}

thread_local! {static HOOK_ID: AtomicUsize = const { AtomicUsize::new(0)}}

impl ModuleContext {
    /// Refer to [create_hook].
    pub fn create_hook<T: Hook + 'static>(&self, hook: T, priority: usize) -> HookHandle {
        let id = HOOK_ID.with(|c| c.fetch_add(1, Ordering::SeqCst));
        let entry = HookEntry {
            id,
            hook: Box::new(hook),
            priority,
        };

        let mut hooks = self.hooks.borrow_mut();
        match hooks.binary_search(&entry) {
            Ok(at) => hooks.insert(at, entry),
            Err(at) => hooks.insert(at, entry),
        };

        HookHandle {
            id,
            #[cfg(debug_assertions)]
            hook_info: format!("{} @ {}", std::any::type_name::<T>(), self.path.path()),
        }
    }

    pub fn destroy_hook(&self, handle: HookHandle) {
        let mut hooks = self.hooks.borrow_mut();
        if let Some((idx, _)) = hooks.iter().enumerate().find(|(_, e)| e.id == handle.id) {
            hooks.remove(idx);
        } else {
            panic!("Hook with id #{} not found on this module", handle.id);
        }
    }
}

// # INTERNALS

pub(crate) struct HookEntry {
    pub(crate) id: usize,
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
