use std::{rc::Rc, sync::Arc};

use crate::{prelude::random, time::Driver};
use tokio::{
    runtime::{Builder, RngSeed, Runtime},
    task::{JoinHandle, LocalSet},
};

use super::ModuleContext;

pub(crate) struct AsyncCoreExt {
    pub(crate) rt: Rt,
    pub(crate) driver: Option<Driver>,

    pub(crate) must_join: Vec<JoinHandle<()>>,
    pub(crate) try_join: Vec<JoinHandle<()>>,
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum Rt {
    Builder(Builder),
    Runtime((Arc<Runtime>, Rc<LocalSet>)),
    Shutdown,
}

impl ModuleContext {
    /// Schedules a task to be joined when the simulatio ends
    ///
    /// This function will **not** block, but rather defer the joining
    /// to the simulation shutdown phase.
    pub fn join(&self, handle: JoinHandle<()>) {
        self.async_ext.write().must_join.push(handle);
    }

    /// Will try to join a task when the simulation ends.
    ///
    /// This will catch panics that occured within the task, but
    /// if the task is still running, no error will be returned.
    pub fn try_join(&self, handle: JoinHandle<()>) {
        self.async_ext.write().try_join.push(handle);
    }

    pub(crate) fn reset_join_handles(&self) {
        self.async_ext.write().must_join.clear();
        self.async_ext.write().try_join.clear();
    }
}

impl AsyncCoreExt {
    pub(crate) fn new() -> AsyncCoreExt {
        #[allow(unused_mut)]
        let mut builder = Builder::new_current_thread();

        #[cfg(feature = "unstable-tokio-enable-time")]
        builder.enable_time();

        Self {
            rt: Rt::Builder(builder),
            driver: Some(Driver::new()),

            must_join: Vec::new(),
            try_join: Vec::new(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.rt = Rt::Runtime((
            Arc::new(
                Builder::new_current_thread()
                    .rng_seed(RngSeed::from_bytes(&random::<u64>().to_le_bytes()))
                    .build()
                    .expect("Failed to build tokio runtime"),
            ),
            Rc::new(LocalSet::new()),
        ));
    }
}

impl Rt {
    pub(crate) fn current(&mut self) -> Option<(Arc<Runtime>, Rc<LocalSet>)> {
        match self {
            Rt::Builder(builder) => {
                let seed = RngSeed::from_bytes(&random::<u64>().to_le_bytes());
                *self = Rt::Runtime((
                    Arc::new(
                        builder
                            .rng_seed(seed)
                            .build()
                            .expect("Failed to build tokio runtime"),
                    ),
                    Rc::new(LocalSet::new()),
                ));
                self.current()
            }
            Rt::Runtime(tupel) => Some(tupel.clone()),
            Rt::Shutdown => None,
        }
    }

    pub(crate) fn shutdown(&mut self) {
        *self = Self::Shutdown;
    }
}
