use des_core::*;
use des_macros::Module;

use log::info;

#[derive(Module)]
#[ndl_workspace = "ndl"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        info!(target: "Alice", "Received at {}: message #{:?} content: {}", sim_time_fmt(),msg.id(), msg.extract_content::<String>());

        self.send(
            Message::new(
                1,
                GATE_NULL,
                self.id(),
                MODULE_NULL,
                SimTime::ZERO,
                String::from("Pong"),
            ),
            ("netOut", 0),
        );

        self.parent_mut::<super::Bob>()
            .unwrap()
            .handle_message(Message::new(
                31,
                GATE_NULL,
                MODULE_NULL,
                MODULE_NULL,
                SimTime::ZERO,
                String::from("Pang"),
            ));
    }
}

impl NdlCompatableModule for Alice {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}

#[derive(Module, Debug)]
#[ndl_workspace = "ndl"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        let id = msg.id();
        let content = msg.extract_content::<String>();
        info!(target: "Bob", "Received at {}: message #{:?} content: {}", sim_time_fmt(), id, content);

        if *content == "Init" {
            self.send(
                Message::new(
                    1,
                    GATE_NULL,
                    self.id(),
                    MODULE_NULL,
                    SimTime::ZERO,
                    String::from("Ping"),
                ),
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
