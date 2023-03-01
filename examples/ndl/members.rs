use std::ops::Deref;

use des::prelude::*;
use log::info;

pub struct Alice();

impl Module for Alice {
    fn new() -> Self {
        Self()
    }

    fn handle_message(&mut self, msg: Message) {
        let mut pkt = msg;
        info!(
            "Received at {}: Message with content: {}",
            SimTime::now(),
            pkt.content::<String>().deref()
        );

        if pkt.header().id > 60_000 {
            // TERMINATE
        } else {
            pkt.header_mut().id += 1;
            send(pkt, ("netOut", 0))
        }
    }
}

pub struct Bob();

impl Module for Bob {
    fn new() -> Self {
        Self()
    }

    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(
            Message::new()
                .kind(0xff)
                .content("Init".to_string())
                .build(),
            Duration::ZERO,
        )
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
                    .id(0)
                    .content("Ping".to_string())
                    .build(),
                ("netOut", 2),
            );
        } else {
            let mut pkt = msg;
            pkt.header_mut().id += 1;

            info!(
                "Received at {}: Message with content: {}",
                SimTime::now(),
                pkt.content::<String>().deref()
            );

            pkt.content_mut::<String>().push('#');

            send(pkt, ("netOut", 2));
        }
    }
}
