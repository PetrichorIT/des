use des::{sim_time, Message, Module, ModuleCore};
use des_derive::Module;
use log::info;

#[derive(Module)]
pub struct Bob(pub ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        let (str, meta) = msg.cast::<String>();
        info!(target: "Bob", "Received at {}: message #{:?} content: {}", sim_time(), meta.id, *str);
    }
}
