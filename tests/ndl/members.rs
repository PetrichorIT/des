use std::ops::Deref;

use des::prelude::*;
use des_derive::Module;

use log::info;

#[derive(Debug, Module)]
#[ndl_workspace = "tests/ndl"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let (mut pkt, _) = msg.cast::<Packet>();
        info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(), pkt.content::<String>().deref());

        if pkt.header().hop_count > 100_000 {
            // TERMINATE
        } else {
            pkt.register_hop();
            self.send(
                Message::new().kind(1).content_interned(pkt).build(),
                ("netOut", 0),
            )
        }
    }
}

#[derive(Module, Debug)]
#[ndl_workspace = "tests/ndl"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        if msg.meta().kind == 0xff {
            info!(target: "Bob", "Initalizing");
            drop(msg);
            info!(target: "Bob", "Dropped init msg");
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
        } else {
            let (mut pkt, _) = msg.cast::<Packet>();
            pkt.register_hop();

            info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(),  pkt.content::<String>().deref());

            pkt.content::<String>().push('#');

            self.send(
                Message::new().kind(1).content_interned(pkt).build(),
                ("netOut", 2),
            );
        }
    }
}
