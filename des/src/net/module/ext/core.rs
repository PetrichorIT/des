use crate::{net::message::Message, time::SimTime};
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

pub(crate) struct AsyncCoreExt {
    pub(crate) rt: Option<std::sync::Arc<tokio::runtime::Runtime>>,

    pub(crate) wait_queue_tx: UnboundedSender<WaitingMessage>,
    pub(crate) wait_queue_rx: Option<UnboundedReceiver<WaitingMessage>>,
    pub(crate) wait_queue_join: Option<JoinHandle<()>>,

    pub(crate) sim_start_tx: UnboundedSender<usize>,
    pub(crate) sim_start_rx: Option<UnboundedReceiver<usize>>,
    pub(crate) sim_start_join: Option<JoinHandle<()>>,

    pub(crate) sim_end_join: Option<JoinHandle<()>>,
}

impl AsyncCoreExt {
    pub(crate) fn new() -> AsyncCoreExt {
        // let (tx, rx) = unbounded_channel();
        let (wtx, wrx) = unbounded_channel();
        let (stx, srx) = unbounded_channel();

        Self {
            rt: Some(std::sync::Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            )),

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
        self.rt = Some(std::sync::Arc::new(tokio::runtime::Runtime::new().unwrap()));

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

#[derive(Debug)]
pub(crate) struct WaitingMessage {
    pub(crate) msg: Message,
    #[allow(dead_code)]
    pub(crate) time: SimTime,
}
