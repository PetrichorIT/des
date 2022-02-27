use des_core::*;
use des_macros::Module;

#[derive(Debug, Module)]
#[ndl_workspace = "des_tests/ptrhell"]
pub struct Alice(ModuleCore);

impl Module for Alice {
    fn at_sim_start(&mut self) {
        let msg = Message::new(
            1,
            GateId::NULL,
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

impl NdlCompatableModule for Alice {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}

#[derive(Debug, Module)]
#[ndl_workspace = "des_tests/ptrhell"]
pub struct Bob(ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        let (msg, head) = msg.cast::<usize>();

        println!("Received msg: {} - {:?}", *msg, head);

        let msg = Message::new_interned(2, self.id(), SimTime::now(), msg);
        self.send(msg, ("netOut", 0))
    }
}

impl NdlCompatableModule for Bob {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}

#[derive(Debug, Module)]
#[ndl_workspace = "des_tests/ptrhell"]
pub struct Network(ModuleCore);

impl Module for Network {
    fn handle_message(&mut self, _: Message) {
        unimplemented!()
    }
}

impl NdlCompatableModule for Network {
    fn named(name: String) -> Self {
        Self(ModuleCore::named(name))
    }
}
