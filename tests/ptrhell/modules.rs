use des::prelude::*;
use des_derive::Module;

#[derive(Debug, Module)]
#[ndl_workspace = "tests/ptrhell"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn at_sim_start(&mut self, _: usize) {
        let msg = Message::new(
            0,
            1,
            None,
            self.id(),
            ModuleId::NULL,
            SimTime::now(),
            42usize,
        );
        self.send(msg, ("netOut", 0));

        println!("SimStared");
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();
        println!("Received msg: {} - {:?}", *msg, head);
    }
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/ptrhell"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();

        println!("Received msg: {} - {:?}", *msg, head);

        let msg = Message::new_interned(0, 2, self.id(), SimTime::now(), msg);
        self.send(msg, ("netOut", 0))
    }
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/ptrhell"]
pub struct Network(ModuleCore);

impl Module for Network {
    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}
