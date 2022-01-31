use des_core::{
    sim_time, Indexable, Message, Module, ModuleCore, SimTime, StaticModuleCore, GATE_NULL,
    MODULE_NULL,
};
use des_macros::Module;

use log::info;

#[derive(Module)]
pub struct Alice(pub ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let (str, meta) = msg.cast::<String>();
        info!(target: "Alice", "Received at {}: message #{:?} content: {}", sim_time(), meta.id, *str);

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
