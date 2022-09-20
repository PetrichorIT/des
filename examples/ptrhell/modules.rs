use des::prelude::*;

#[derive(Debug)]
#[NdlModule("examples/ptrhell")]
pub struct Alice();

impl Module for Alice {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _: usize) {
        let msg = Message::new().kind(1).content(42usize).build();
        send(msg, ("netOut", 0));

        println!("SimStared");
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();
        println!("Received msg: {} - {:?}", msg, head);
    }
}

#[derive(Debug)]
#[NdlModule("examples/ptrhell")]
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
#[NdlModule("examples/ptrhell")]
pub struct Network();

impl Module for Network {
    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}
