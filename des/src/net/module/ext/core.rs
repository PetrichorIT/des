use std::sync::Arc;

use crate::{
    net::message::Message,
    prelude::random,
    time::{Driver, SimTime},
};
use tokio::{
    runtime::{Builder, RngSeed, Runtime},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::{JoinHandle, LocalSet},
};

pub(crate) struct AsyncCoreExt {
    pub(crate) rt: Rt,
    pub(crate) driver: Option<Driver>,

    pub(crate) wait_queue_tx: UnboundedSender<WaitingMessage>,
    pub(crate) wait_queue_rx: Option<UnboundedReceiver<WaitingMessage>>,
    pub(crate) wait_queue_join: Option<JoinHandle<()>>,

    pub(crate) sim_start_tx: UnboundedSender<usize>,
    pub(crate) sim_start_rx: Option<UnboundedReceiver<usize>>,
    pub(crate) sim_start_join: Option<JoinHandle<()>>,

    pub(crate) sim_end_join: Option<JoinHandle<()>>,
}

pub(crate) enum Rt {
    Builder(Builder),
    Runtime((Arc<Runtime>, Arc<LocalSet>)),
    Shutdown,
}

impl AsyncCoreExt {
    pub(crate) fn new() -> AsyncCoreExt {
        // let (tx, rx) = unbounded_channel();
        let (wtx, wrx) = unbounded_channel();
        let (stx, srx) = unbounded_channel();

        #[allow(unused_mut)]
        let mut builder = Builder::new_current_thread();

        #[cfg(feature = "unstable-tokio-enable-time")]
        builder.enable_time();

        Self {
            rt: Rt::Builder(builder),

            driver: Some(Driver::new()),

            wait_queue_tx: wtx,
            wait_queue_rx: Some(wrx),
            wait_queue_join: None,

            sim_start_tx: stx,
            sim_start_rx: Some(srx),
            sim_start_join: None,

            sim_end_join: None,
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
            Arc::new(LocalSet::new()),
        ));

        // let (tx, rx) = unbounded_channel();
        let (wtx, wrx) = unbounded_channel();
        let (stx, srx) = unbounded_channel();

        // self.buffers = rx;
        // self.handle = tx;

        self.wait_queue_tx = wtx;
        self.wait_queue_rx = Some(wrx);
        self.wait_queue_join = None;

        self.sim_start_tx = stx;
        self.sim_start_rx = Some(srx);
        self.sim_start_join = None;

        self.sim_end_join = None;
    }
}

impl Rt {
    pub(crate) fn current(&mut self) -> Option<(Arc<Runtime>, Arc<LocalSet>)> {
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
                    Arc::new(LocalSet::new()),
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

#[derive(Debug)]
pub(crate) struct WaitingMessage {
    pub(crate) msg: Message,
    #[allow(dead_code)]
    pub(crate) time: SimTime,
}
