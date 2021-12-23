use des_core::*;
use des_macros::Module;

#[derive(Module)]
#[ndl_workspace = "./ndl"]
struct Alice(ModuleCore);

impl Module for Alice {
    fn handle_message(&mut self, msg: Message) {}
}
