use crate::net::module::ModuleContext;
use std::{
    any::Any,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    sync::atomic::Ordering,
};

pub(super) struct SimWideUnwind(pub(super) Box<dyn Any + Send + 'static>);

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

    pub(super) fn catch(self) {
        if let Some(unwind) = self.unwind {
            display_panic(&unwind);

            self.ctx.active.store(false, Ordering::SeqCst);
            match unwind.downcast::<SimWideUnwind>() {
                Ok(sim_unwind) => resume_unwind(sim_unwind.0),
                Err(other_unwind) if !self.ctx.stereotyp.get().on_panic_catch => {
                    resume_unwind(other_unwind)
                }
                _ => {}
            }
        }
    }

    pub(super) fn pass(self) {
        if let Some(e) = self.unwind {
            display_panic(&e);
            resume_unwind(e);
        }
    }
}

#[cfg(feature = "tracing")]
#[inline]
fn display_panic(unwind: &Box<dyn Any + Send + 'static>) {
    if let Some(str) = unwind.downcast_ref::<&str>() {
        tracing::error!("module paniced: \n{str}");
    }
    println!("{:?}", unwind.type_id())
}

#[cfg(not(feature = "tracing"))]
#[inline]
fn display_panic(_: &Box<dyn Any + Send + 'static>) {}
