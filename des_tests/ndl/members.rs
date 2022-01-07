use std::ops::Deref;

use des_core::*;
use des_macros::Module;

use log::info;

#[derive(Module)]
#[ndl_workspace = "des_tests/ndl"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let mut pkt = msg.extract_content::<Packet>();
        info!(target: self.name().unwrap(), "Received at {}: Message #{} content: {}", sim_time_fmt(), pkt.id(), pkt.extract_content_ref::<String>().deref());

        if pkt.hop_count() > 100_000 {
            // TERMINATE
        } else {
            pkt.inc_hop_count();
            self.send(
                Message::new_interned(1, self.id(), SimTime::ZERO, pkt),
                ("netOut", 0),
            )
        }
    }
}

impl NdlCompatableModule for Alice {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}

#[derive(Module, Debug)]
#[ndl_workspace = "des_tests/ndl"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        if msg.kind == 0xff {
            info!(target: "Bob", "Initalizing");
            drop(msg);
            info!(target: "Bob", "Dropped init msg");
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
        } else {
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
}

impl NdlCompatableModule for Bob {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}
