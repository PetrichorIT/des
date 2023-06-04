use des::prelude::*;

#[derive(Debug)]
pub struct Alice();

impl Module for Alice {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _: usize) {
        let msg = Message::new().kind(1).content(42usize).build();
        send(msg, ("netOut", 0));

        tracing::info!("SimStared");
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();
        tracing::info!(target: "inet", "Received msg: {} - {:?}", msg, head);
    }
}

#[derive(Debug)]
pub struct Bob();

impl Module for Bob {
    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();

        println!("Received msg: {} - {:?}", msg, head);

        let msg = Message::new().kind(2).content(msg).build();
        send(msg, ("netOut", 0))
    }
}

#[derive(Debug)]
pub struct Network();

impl Module for Network {
    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}
