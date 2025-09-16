use des::{
    net::{
        handlers::ModuleFn,
        module::{SIGNAL_MODULE_PANICED, Signal, Stereotyp},
    },
    prelude::*,
};
use serial_test::serial;

struct Parent(bool);
impl Module for Parent {
    fn at_sim_start(&mut self, _stage: usize) {
        let child = current().child("child").unwrap();
        child.set_stereotyp(Stereotyp::SUBPROCESS);
        child.subscribe_to(SIGNAL_MODULE_PANICED);
    }
    fn handle_signal(&mut self, signal: Signal) {
        match signal.code {
            SIGNAL_MODULE_PANICED => {
                // Handle the panic signal
                self.0 = true;
            }
            _ => {}
        }
    }
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert!(self.0, "Must have observed child's panic");
        Ok(())
    }
}

#[test]
#[serial]
fn parent_observes_childs_panic() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node("parent", Parent(false));
    sim.node(
        "parent.child",
        ModuleFn::new(
            || schedule_at(Message::default(), 2.0.into()),
            |_, _| panic!("some reason"),
        ),
    );

    Builder::seeded(123).build(sim.freeze()).run().map(|_| ())
}
