use des_core::{sim_time_fmt, Message, Module, ModuleCore};
use log::info;

pub struct Bob(pub ModuleCore);

impl Module for Bob {
    fn module_core(&self) -> &ModuleCore {
        &self.0
    }

    fn module_core_mut(&mut self) -> &mut ModuleCore {
        &mut self.0
    }

    fn handle_message(&mut self, msg: Message) {
        info!(target: "Bob", "Received at {}: message #{:?} content: {}", sim_time_fmt(),msg.id(), msg.extract_content::<String>());
    }
}
