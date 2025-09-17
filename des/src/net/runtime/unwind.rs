use crate::{
    net::{
        Error, ErrorKind,
        message::Body,
        module::{ModuleContext, SIGNAL_MODULE_PANICED, State, emit},
        runtime::{ModuleRestartEvent, NetEvents},
        schedule_event,
    },
    time::SimTime,
};
use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind},
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

    pub(super) fn exec(mut self, f: impl FnOnce()) -> Self {
        self.unwind = catch_unwind(AssertUnwindSafe(f)).err();
        self
    }

    pub(super) fn catch(self) -> Result<(), Error> {
        if let Some(unwind) = self.unwind {
            let bh = self.ctx.unwind_behaviour();

            self.ctx.state.set(State::Shutdown);

            emit(SIGNAL_MODULE_PANICED, Body::empty());

            if !bh.on_panic_catch {
                return Err(Error::new(self.ctx.path(), ErrorKind::ModulePanic(unwind)));
            }

            if bh.on_panic_restart {
                schedule_event(
                    NetEvents::ModuleRestartEvent(ModuleRestartEvent {
                        module: self.ctx.me(),
                    }),
                    SimTime::now(),
                );
            }

            if bh.on_panic_drop_submodules {
                // TODO: impl drop submodules
            }
        }
        Ok(())
    }

    pub(super) fn pass(self) -> Result<(), Error> {
        if let Some(unwind) = self.unwind {
            return Err(Error::new(self.ctx.path(), ErrorKind::ModulePanic(unwind)));
        }
        Ok(())
    }
}
