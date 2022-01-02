use std::ops::Deref;

use des_core::*;
use des_macros::Module;

use log::{info, warn};

#[derive(Module)]
#[ndl_workspace = "util"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let mut pkt = msg.extract_content::<Packet>();
        info!(target: self.name().unwrap(), "Received at {}: Message #{} content: {}", sim_time_fmt(), pkt.id(), pkt.extract_content_ref::<String>().deref());

        if pkt.hop_count() > 4 {
            // TERMINATE
            self.disable_activity()
        } else {
            pkt.inc_hop_count();
            self.send(
                Message::new_interned(1, self.id(), SimTime::ZERO, pkt),
                ("netOut", 0),
            )
        }
    }

    fn at_simulation_start(&mut self) {
        self.enable_activity(SimTime::from(3.0));
    }

    fn activity(&mut self) {
        warn!(target: &self.str(), "ACTIVITY");
    }
}

impl NdlCompatableModule for Alice {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}

#[derive(Module, Debug)]
#[ndl_workspace = "util"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn at_simulation_start(&mut self) {
        info!(target: "Bob", "Initalizing");
        self.send(
            Message::new(
                1,
                GATE_SELF,
                self.id(),
                MODULE_NULL,
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
        let mut pkt = msg.extract_content::<Packet>();
        pkt.inc_hop_count();

        info!(target: self.name().unwrap(), "Received at {}: Message #{} content: {}", sim_time_fmt(), pkt.id(), pkt.extract_content_ref::<String>().deref());

        pkt.extract_content_ref::<String>().push('#');

        self.send(
            Message::new_interned(1, self.id(), SimTime::ZERO, pkt),
            ("netOut", 2),
        );
    }
}

impl NdlCompatableModule for Bob {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}
