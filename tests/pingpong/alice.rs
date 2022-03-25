use des::prelude::*;
use des_derive::Module;

use log::info;

#[derive(Module)]
pub struct Alice(pub ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let (str, meta) = msg.cast::<String>();
        info!(target: "Alice", "Received at {}: message #{:?} content: {}", sim_time(), meta.id, *str);

        self.send(
            Message::new(
                0,
                1,
                None,
                self.id(),
                ModuleId::NULL,
                SimTime::ZERO,
                String::from("Pong"),
            ),
            ("netOut", 0),
        );

        self.parent_mut::<super::bob::Bob>()
            .unwrap()
            .handle_message(Message::new(
                0,
                31,
                None,
                ModuleId::NULL,
                ModuleId::NULL,
                SimTime::ZERO,
                String::from("Pang"),
            ));
    }
}
