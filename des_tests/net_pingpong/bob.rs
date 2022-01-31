use des_core::{sim_time, Message, Module, ModuleCore};
use des_macros::Module;
use log::info;
use std::ops::Deref;

#[derive(Module)]
pub struct Bob(pub ModuleCore);

impl Module for Bob {
    fn handle_message(&mut self, msg: Message) {
        info!(target: "Bob", "Received at {}: message #{:?} content: {}", sim_time(),msg.id(), msg.cast::<String>().0.deref());
    }
}
