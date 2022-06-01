use tokio::sync::Notify;

use crate::prelude::*;
use tokio::runtime::Runtime;

pub mod sync;

pub const P_WAKEUP: MessageId = 1;
pub const P_PACKET: MessageId = 2;

pub struct AsyncCore {
    to_worker: sync::mpsc::Sender<Packet>,
    notifier: Notify,

    runtime: Runtime,
    periodic_callbacks: Vec<(fn() -> (), Duration)>,
}

impl AsyncCore {
    pub fn new() -> Self {
        todo!();
        // Self {
        //     runtime: Runtime::new().unwrap(),
        //     periodic_callbacks: Vec::new(),
        // }
    }

    pub fn handle_message(&mut self, message: Message) {
        match message.meta().id {
            P_WAKEUP => {
                let (p_content, _) = message.cast::<usize>();
                self.periodic_callbacks[p_content].0()

                // self.shedule_at(self.periodic_callbacks[p_content].0, Message::new().id(P_WAKEUP).content(p_content).build())
            }
            P_PACKET => {
                let pkt = message.as_packet();
                self.to_worker.blocking_send(pkt).unwrap();

                // await the results
                self.runtime.block_on(self.notifier.notified())
            }

            _ => panic!("Unknown packet ID"),
        }
    }
}
