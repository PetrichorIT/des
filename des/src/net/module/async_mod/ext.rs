use crate::{net::*, time::SimTime};
use tokio::{sync::mpsc::*, task::JoinHandle, time::TimeContext};

pub(crate) struct AsyncCoreExt {
    pub(crate) buffers: UnboundedReceiver<super::BufferEvent>,
    pub(crate) handle: UnboundedSender<super::BufferEvent>,
    pub(crate) time_context: Option<TimeContext>,

    pub(crate) wait_queue_tx: UnboundedSender<WaitingMessage>,
    pub(crate) wait_queue_rx: Option<UnboundedReceiver<WaitingMessage>>,
    pub(crate) wait_queue_join: Option<JoinHandle<()>>,

    pub(crate) sim_start_tx: UnboundedSender<usize>,
    pub(crate) sim_start_rx: Option<UnboundedReceiver<usize>>,
    pub(crate) sim_start_join: Option<JoinHandle<()>>,

    pub(crate) sim_end_join: Option<JoinHandle<()>>,
}

impl AsyncCoreExt {
    pub(crate) fn new(ident: String) -> AsyncCoreExt {
        let (tx, rx) = unbounded_channel();
        let (wtx, wrx) = unbounded_channel();
        let (stx, srx) = unbounded_channel();

        Self {
            buffers: rx,
            handle: tx,

            time_context: Some(TimeContext::new(ident)),

            wait_queue_tx: wtx,
            wait_queue_rx: Some(wrx),
            wait_queue_join: None,

            sim_start_tx: stx,
            sim_start_rx: Some(srx),
            sim_start_join: None,

            sim_end_join: None,
        }
    }
}

#[derive(Debug)]
#[allow(unused)]
pub(crate) struct WaitingMessage {
    pub(crate) msg: Message,
    pub(crate) time: SimTime,
}
