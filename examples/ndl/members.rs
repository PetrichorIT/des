use std::ops::Deref;

use des::prelude::*;
use log::info;

#[NdlModule("examples/ndl")]
pub struct Alice();

impl Module for Alice {
    fn new() -> Self {
        Self()
    }

    fn handle_message(&mut self, msg: Message) {
        let mut pkt = msg;
        info!(
            "Received at {}: Message with content: {}",
            sim_time(),
            pkt.content::<String>().deref()
        );

        if pkt.header().hop_count > 100_000 {
            // TERMINATE
        } else {
            pkt.register_hop();
            send(pkt, ("netOut", 0))
        }
    }
}

#[NdlModule("examples/ndl")]
pub struct Bob();

impl Module for Bob {
    fn new() -> Self {
        Self()
    }

    fn handle_message(&mut self, msg: Message) {
        if msg.header().kind == 0xff {
            info!(target: "Bob", "Initalizing");
            drop(msg);
            info!(target: "Bob", "Dropped init msg");
            send(
                Message::new()
                    .kind(1)
                    // .src(0x7f_00_00_01, 80)
                    // .dest(0x7f_00_00_02, 80)
                    .content("Ping".to_string())
                    .build(),
                ("netOut", 2),
            );
        } else {
            let mut pkt = msg;
            pkt.register_hop();

            info!(
                "Received at {}: Message with content: {}",
                sim_time(),
                pkt.content::<String>().deref()
            );

            pkt.content_mut::<String>().push('#');

            send(pkt, ("netOut", 2));
        }
    }
}
