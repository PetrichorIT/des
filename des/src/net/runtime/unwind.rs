use crate::net::{Error, ErrorKind, module::ModuleContext};
use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind},
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

    pub(super) fn catch(self) -> Result<(), Error> {
        if let Some(unwind) = self.unwind {
            // display_panic(&unwind);

            self.ctx.active.store(false, Ordering::SeqCst);
            if !self.ctx.stereotyp.get().on_panic_catch {
                return Err(Error {
                    origin: self.ctx.path(),
                    kind: ErrorKind::ModulePanic(unwind),
                });
            }
        }
        Ok(())
    }

    pub(super) fn pass(self) -> Result<(), Error> {
        if let Some(unwind) = self.unwind {
            return Err(Error {
                origin: self.ctx.path(),
                kind: ErrorKind::ModulePanic(unwind),
            });
        }
        Ok(())
    }
}
