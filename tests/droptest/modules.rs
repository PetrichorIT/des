use des::prelude::*;

use crate::MODULE_LEN;
#[derive(Debug, Module)]
#[ndl_workspace = "tests/ptrhell"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn at_sim_start(&mut self, _: usize) {
        let msg = Message::new().kind(1).content(42usize).build();
        self.send(msg, ("netOut", 0));

        println!("SimStared");
        *MODULE_LEN.lock().unwrap() += 1;
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();
        println!("Received msg: {} - {:?}", msg, head);
    }
}

impl Drop for Alice {
    fn drop(&mut self) {
        *MODULE_LEN.lock().unwrap() -= 1;
    }
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/ptrhell"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn at_sim_start(&mut self, _stage: usize) {
        *MODULE_LEN.lock().unwrap() += 1;
    }

    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();

        println!("Received msg: {} - {:?}", msg, head);

        let msg = Message::new().kind(2).content(msg).build();
        self.send(msg, ("netOut", 0))
    }
}

impl Drop for Bob {
    fn drop(&mut self) {
        *MODULE_LEN.lock().unwrap() -= 1;
    }
}

#[derive(Debug, Module)]
#[ndl_workspace = "tests/ptrhell"]
pub struct Network(ModuleCore);

impl Module for Network {
    fn at_sim_start(&mut self, _: usize) {
        *MODULE_LEN.lock().unwrap() += 1;
    }

    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        *MODULE_LEN.lock().unwrap() -= 1;
    }
}
