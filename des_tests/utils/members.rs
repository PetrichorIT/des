use std::ops::Deref;

use des_core::*;
use des_macros::Module;

use log::{info, warn};

#[derive(Module)]
#[ndl_workspace = "des_tests/utils"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let (mut pkt, _) = msg.cast::<Packet>();
        info!(target: self.name(), "Received at {}: Message #{} content: {}", sim_time(), pkt.id(), pkt.content::<String>().deref());

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

    fn at_sim_start(&mut self) {
        self.enable_activity(SimTime::from(3.0));
    }

    fn activity(&mut self) {
        warn!(target: self.str(), "ACTIVITY");
    }
}

#[derive(Module, Debug)]
#[ndl_workspace = "des_tests/utils"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn at_sim_start(&mut self) {
        info!(target: "Bob", "Initalizing");
        self.send(
            Message::new(
                0,
                1,
                GateId::NULL,
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

    fn handle_message(&mut self, msg: Message) {
        let (mut pkt, _) = msg.cast::<Packet>();
        pkt.inc_hop_count();

        info!(target: self.name(), "Received at {}: Message #{} content: {}", sim_time(), pkt.id(), pkt.content::<String>().deref());

        pkt.content::<String>().push_str(&self.pars()["char"]);

        self.send(
            Message::new_interned(0, 1, self.id(), SimTime::ZERO, pkt),
            ("netOut", 2),
        );
    }
}
