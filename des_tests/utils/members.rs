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
        info!(target: self.name().unwrap(), "Received at {}: Message #{} content: {}", sim_time(), pkt.id(), pkt.extract_content_ref::<String>().deref());

        if pkt.hop_count() > 2 {
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

    fn at_sim_start(&mut self) {
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
#[ndl_workspace = "des_tests/utils"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn at_sim_start(&mut self) {
        info!(target: "Bob", "Initalizing");
        self.send(
            Message::new(
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

        info!(target: self.name().unwrap(), "Received at {}: Message #{} content: {}", sim_time(), pkt.id(), pkt.extract_content_ref::<String>().deref());

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
