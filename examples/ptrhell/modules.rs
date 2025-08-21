use des::prelude::*;

#[derive(Debug, Default)]
pub struct Alice();

impl Module for Alice {
    fn at_sim_start(&mut self, _: usize) {
        let msg = Message::default().with_kind(1).with_content(42usize);
        let _ = send(msg, ("netOut", 0));

        tracing::info!("SimStared");
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.into_content::<usize>();
        tracing::info!(target: "inet", "Received msg: {} - {:?}", msg, head);
    }
}

#[derive(Debug, Default)]
pub struct Bob();

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.into_content::<usize>();

        println!("Received msg: {} - {:?}", msg, head);

        let msg = Message::default().with_kind(2).with_content(msg);
        let _ = send(msg, ("netOut", 0));
    }
}

#[derive(Debug, Default)]
pub struct Network();

impl Module for Network {
    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}
