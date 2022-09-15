use std::ops::Deref;

use des::prelude::*;

use log::info;

#[derive(Debug)]
#[NdlModule("examples/utils")]
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

        if pkt.header().hop_count > par("limit").unwrap().parse::<usize>().unwrap() {
            // TERMINATE
        } else {
            pkt.register_hop();
            send(pkt, ("netOut", 0))
        }
    }
}

#[derive(Debug)]
#[NdlModule("examples/utils")]
pub struct Bob();

impl Module for Bob {
    fn new() -> Self {
        Self()
    }

    fn num_sim_start_stages(&self) -> usize {
        2
    }

    fn at_sim_start(&mut self, stage: usize) {
        match stage {
            0 => {
                info!("Initalizing");
                send(
                    Message::new()
                        .kind(1)
                        // .src(0x7f_00_00_01, 80)
                        // .dest(0x7f_00_00_02, 80)
                        .content("Ping".to_string())
                        .build(),
                    ("netOut", 2),
                );
            }
            1 => {
                // Nothing
            }
            _ => unreachable!(),
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let mut pkt = msg;
        pkt.register_hop();

        info!(
            "Received at {}: Message with content: {}",
            sim_time(),
            pkt.content::<String>().deref()
        );

        pkt.content_mut::<String>().push_str(&par("char").unwrap());

        send(pkt, ("netOut", 2));
    }
}
