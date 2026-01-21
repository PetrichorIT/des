#![allow(unused)]

use des::{Error, module::SIGNAL_MODULE_PANICED, prelude::*};

pub struct NopModule;
impl Module for NopModule {}

pub struct ExpectNMessage(pub i32);

impl Module for ExpectNMessage {
    fn handle_message(&mut self, _msg: Message) {
        self.0 -= 1;
    }

    fn at_sim_end(&mut self) -> Result<(), Error> {
        assert_eq!(self.0, 0, "expected {} more messages", self.0);
        Ok(())
    }
}
