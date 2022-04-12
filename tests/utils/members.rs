use std::ops::Deref;

use des::prelude::*;
use des_derive::Module;

use log::{info, warn};

#[derive(Debug, Module)]
#[ndl_workspace = "tests/utils"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let (mut pkt, _) = msg.cast::<Packet>();
        info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(), pkt.content::<String>().deref());

        if pkt.header().hop_count > self.par("limit").unwrap().parse::<usize>().unwrap() {
            // TERMINATE
            self.disable_activity()
        } else {
            pkt.register_hop();
            self.send(
                Message::new().kind(1).content_interned(pkt).build(),
                ("netOut", 0),
            )
        }
    }

    fn at_sim_start(&mut self, _: usize) {
        self.enable_activity(SimTime::from(3.0));
    }

    fn activity(&mut self) {
        warn!(target: self.str(), "ACTIVITY");
    }
}

#[derive(Module, Debug)]
#[ndl_workspace = "tests/utils"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn num_sim_start_stages(&self) -> usize {
        2
    }

    fn at_sim_start(&mut self, stage: usize) {
        match stage {
            0 => {
                info!(target: self.str(), "Initalizing");
                self.send(
                    Message::new()
                        .kind(1)
                        .content(
                            Packet::new()
                                .src(0x7f_00_00_01, 80)
                                .dest(0x7f_00_00_02, 80)
                                .content("Ping".to_string())
                                .build(),
                        )
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
        let (mut pkt, _) = msg.cast::<Packet>();
        pkt.register_hop();

        info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(), pkt.content::<String>().deref());

        pkt.content::<String>().push_str(&self.par("char").unwrap());

        self.send(
            Message::new().kind(1).content_interned(pkt).build(),
            ("netOut", 2),
        );
    }
}
