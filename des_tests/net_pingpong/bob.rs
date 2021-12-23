use des_core::{sim_time_fmt, Message, Module, ModuleCore};
use des_macros::Module;
use log::info;

#[derive(Module)]
pub struct Bob(pub ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        info!(target: "Bob", "Received at {}: message #{:?} content: {}", sim_time_fmt(),msg.id(), msg.extract_content::<String>());
    }
}
