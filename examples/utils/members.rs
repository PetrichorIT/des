use std::ops::Deref;

use des::prelude::*;

use log::info;

#[derive(Debug)]
#[NdlModule("examples/utils")]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let mut pkt = msg.as_packet();
        info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(), pkt.content::<String>().deref());

        if pkt.header().hop_count > self.par("limit").unwrap().parse::<usize>().unwrap() {
            // TERMINATE
            self.disable_activity()
        } else {
            pkt.register_hop();
            self.send(pkt, ("netOut", 0))
        }
    }

    fn at_sim_start(&mut self, _: usize) {
        self.enable_activity(Duration::from(3.0));
    }

    fn activity(&mut self) {
        info!(target: self.str(), "ACTIVITY");
    }
}

#[derive(Debug)]
#[NdlModule("examples/utils")]
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
                    Packet::new()
                        .kind(1)
                        .src(0x7f_00_00_01, 80)
                        .dest(0x7f_00_00_02, 80)
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
        let mut pkt = msg.as_packet();
        pkt.register_hop();

        info!(target: self.name(), "Received at {}: Message with content: {}", sim_time(), pkt.content::<String>().deref());

        pkt.content_mut::<String>()
            .push_str(&self.par("char").unwrap());

        self.send(pkt, ("netOut", 2));
    }
}
