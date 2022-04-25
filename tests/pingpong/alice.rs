use des::prelude::*;

use log::info;

#[derive(Module)]
pub struct Alice(pub ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {
        let (str, meta) = msg.cast::<String>();
        info!(target: "Alice", "Received at {}: message #{:?} content: {}", sim_time(), meta.id, str);

        self.send(
            Message::new().content("Pong".to_string()).build(),
            ("netOut", 0),
        );

        self.parent_mut::<super::bob::Bob>()
            .unwrap()
            .handle_message(Message::new().kind(31).content("Pang".to_string()).build());
    }
}
