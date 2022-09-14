use des::prelude::*;

use log::info;

#[NdlModule]
pub struct Alice();

impl Module for Alice {
    fn new() -> Self {
        Alice()
    }

    fn handle_message(&mut self, msg: Message) {
        let (str, meta) = msg.cast::<String>();
        info!(target: "Alice", "Received at {}: message #{:?} content: {}", sim_time(), meta.id, str);

        send(
            Message::new().content("Pong".to_string()).build(),
            ("netOut", 0),
        );

        parent()
            .unwrap()
            .handle_message(Message::new().kind(31).content("Pang".to_string()).build());
    }
}
