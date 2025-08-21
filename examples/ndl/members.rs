use std::ops::Deref;

use des::prelude::*;
use tracing::info;

#[derive(Default)]
pub struct Alice();

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let mut pkt = msg;
        info!(
            "Received at {}: Message with content: {}",
            SimTime::now(),
            pkt.body.content::<String>().deref()
        );

        if pkt.header().id > 60_000 {
            // TERMINATE
        } else {
            pkt.header_mut().id += 1;
            let _ = send(pkt, ("netOut", 0));
        }
    }
}

#[derive(Default)]
pub struct Bob();

impl Module for Bob {
    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(
            Message::default()
                .with_kind(0xff)
                .with_content("Init".to_string()),
            Duration::ZERO,
        )
    }

    fn handle_message(&mut self, msg: Message) {
        if msg.header().kind == 0xff {
            info!(target: "Bob", "Initalizing");
            drop(msg);
            info!(target: "Bob", "Dropped init msg");
            let _ = send(
                Message::default()
                    .with_kind(1)
                    // .src(0x7f_00_00_01, 80)
                    // .dest(0x7f_00_00_02, 80)
                    .with_id(0)
                    .with_content("Ping".to_string()),
                ("netOut", 2),
            );
        } else {
            let mut pkt = msg;
            pkt.header_mut().id += 1;

            info!(
                "Received at {}: Message with content: {}",
                SimTime::now(),
                pkt.body.content::<String>().deref()
            );

            pkt.body.content_mut::<String>().push('#');

            let _ = send(pkt, ("netOut", 2));
        }
    }
}
