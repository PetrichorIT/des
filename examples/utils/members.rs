use std::ops::Deref;

use des::prelude::*;

use tracing::info;

#[derive(Debug, Default)]
pub struct Alice();

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let pkt = msg;
        info!(
            "Received at {}: Message with content: {}",
            SimTime::now(),
            pkt.content::<String>().deref()
        );

        let limit = current().prop::<usize>("limit").unwrap().or_default().get();
        if pkt.content::<String>().len() > limit {
            // TERMINATE
        } else {
            // pkt.content_mut::<String>().push('#');
            send(pkt, ("netOut", 0))
        }
    }
}

#[derive(Debug, Default)]
pub struct Bob();

impl Module for Bob {
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

        info!(
            "Received at {}: Message with content: {}",
            SimTime::now(),
            pkt.content::<String>().deref()
        );

        pkt.content_mut::<String>()
            .push_str(&current().prop::<String>("char").unwrap().get().unwrap());

        send(pkt, ("netOut", 2));
    }
}
