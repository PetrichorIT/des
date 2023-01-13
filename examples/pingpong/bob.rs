use des::prelude::*;
use log::info;

#[NdlModule]
pub struct Bob();

impl Module for Bob {
    fn new() -> Bob {
        Bob()
    }

    fn handle_message(&mut self, msg: Message) {
        let (str, meta) = msg.cast::<String>();
        info!(target: "Bob", "Received at {}: message #{:?} content: {}", SimTime::now(), meta.id, str);

        if str == "Pong" {
            let msg = Message::new().content("Ping".to_string()).build();

            send(msg, ("netOut", 0))
        }
    }
}
