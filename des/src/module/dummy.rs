use crate::{
    Error,
    prelude::{Message, Module},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DummyModule;

impl Module for DummyModule {
    fn handle_message(&mut self, _msg: Message) {
        panic!(
            "A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called."
        )
    }

    fn handle_signal(&mut self, _signal: super::Signal) {
        panic!(
            "A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called."
        )
    }

    fn at_sim_start(&mut self, _stage: usize) {
        panic!(
            "A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called."
        )
    }

    fn num_sim_start_stages(&self) -> usize {
        panic!(
            "A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called."
        )
    }

    fn at_sim_end(&mut self) -> Result<(), Error> {
        panic!(
            "A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called."
        )
    }
}
