use std::ops::Deref;

use des::prelude::*;
use des_derive::Module;

use log::{info, warn};

#[derive(Module)]
#[ndl_workspace = "tests/utils"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let (mut pkt, _) = msg.cast::<Packet>();
        info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(), pkt.content::<String>().deref());

        if pkt.header().hop_count > self.pars()["limit"].parse::<usize>().unwrap() {
            // TERMINATE
            self.disable_activity()
        } else {
            pkt.inc_hop_count();
            self.send(
                Message::new_interned(0, 1, self.id(), SimTime::ZERO, pkt),
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
                    Message::new(
                        0,
                        1,
                        None,
                        self.id(),
                        ModuleId::NULL,
                        SimTime::now(),
                        Packet::new(
                            (0x7f_00_00_01, 80),
                            (0x7f_00_00_02, 80),
                            String::from("Ping"),
                        ),
                    ),
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
        pkt.inc_hop_count();

        info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(), pkt.content::<String>().deref());

        pkt.content::<String>().push_str(&self.pars()["char"]);

        self.send(
            Message::new_interned(0, 1, self.id(), SimTime::ZERO, pkt),
            ("netOut", 2),
        );
    }
}
