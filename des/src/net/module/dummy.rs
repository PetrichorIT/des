use super::Module;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DummyModule {}

impl Module for DummyModule {
    fn new() -> Self {
        panic!("A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called.")
    }

    fn handle_message(&mut self, _msg: crate::prelude::Message) {
        panic!("A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called.")
    }

    fn handle_par_change(&mut self) {
        panic!("A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called.")
    }

    fn at_sim_start(&mut self, _stage: usize) {
        panic!("A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called.")
    }

    fn num_sim_start_stages(&self) -> usize {
        panic!("A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called.")
    }

    fn at_sim_end(&mut self) {
        panic!("A dummy module is only a placeholder in the load process. No `dyn Module` functions should be called.")
    }
}
