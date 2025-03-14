use crate::net::{module::ModuleContext, ObjectPath};
use std::{
    any::Any,
    error::Error as StdError,
    fmt::{Debug, Display},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::atomic::Ordering,
};

#[must_use]
pub(super) struct Harness<'a> {
    ctx: &'a ModuleContext,
    unwind: Option<Box<dyn Any + Send + 'static>>,
}

impl<'a> Harness<'a> {
    pub(super) fn new(ctx: &'a ModuleContext) -> Self {
        Harness { ctx, unwind: None }
    }

    #[cfg(not(feature = "async"))]
    pub(super) fn exec(mut self, f: impl FnOnce()) -> Self {
        self.unwind = catch_unwind(AssertUnwindSafe(|| f())).err();
        self
    }

    #[cfg(feature = "async")]
    pub(super) fn exec(mut self, f: impl FnOnce()) -> Self {
        let Some((rt, task_set)) = self.ctx.async_ext.write().rt.current() else {
            panic!("simulation error: tokio runtime was lost during execution");
        };

        self.unwind = catch_unwind(AssertUnwindSafe(|| {
            task_set.block_on(&rt, async move {
                f();
                tokio::task::yield_now().await;
            });
        }))
        .err();
        self
    }

    pub(super) fn catch(self) -> Result<(), PanicError> {
        if let Some(unwind) = self.unwind {
            // display_panic(&unwind);

            self.ctx.active.store(false, Ordering::SeqCst);
            if !self.ctx.stereotyp.get().on_panic_catch {
                return Err(PanicError {
                    path: self.ctx.path(),
                    payload: unwind,
                });
            }
        }
        Ok(())
    }

    pub(super) fn pass(self) -> Result<(), PanicError> {
        if let Some(unwind) = self.unwind {
            return Err(PanicError {
                path: self.ctx.path(),
                payload: unwind,
            });
        }
        Ok(())
    }
}

/// An non-catchable panic occured.
pub struct PanicError {
    /// The source of the panic
    pub path: ObjectPath,
    /// The panic payload itself
    pub payload: Box<dyn Any + Send>,
}

impl Debug for PanicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PanicError")
            .field("path", &self.path.as_str())
            .field("payload", &self.payload)
            .finish()
    }
}

impl Display for PanicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "module '{}' panicked", self.path)
    }
}

impl StdError for PanicError {}
