use dse::{sim_time_fmt, Message, Module, ModuleCore, ModuleExt, SimTime, GATE_NULL, MODULE_NULL};
use log::info;

pub struct Alice(pub ModuleCore);

impl Module for Alice {
    fn module_core(&self) -> &ModuleCore {
        &self.0
    }

    fn module_core_mut(&mut self) -> &mut ModuleCore {
        &mut self.0
    }

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

        self.parent_mut::<super::bob::Bob>()
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

impl ModuleExt for Alice {}
