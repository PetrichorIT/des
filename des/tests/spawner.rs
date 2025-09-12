use des::{
    net::{globals, handlers::ModuleFn, processing::ProcessingStack},
    prelude::*,
};
use serial_test::serial;

struct WithSimStartRequired(bool);
impl Module for WithSimStartRequired {
    fn at_sim_start(&mut self, _stage: usize) {
        self.0 = true;
    }
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert!(self.0, "must be set by at_sim_start");
        Ok(())
    }
}

#[test]
#[serial]
fn runtime_spawner_calls_sim_start() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::new(
            || {
                schedule_at(Message::default().with_id(1), 1.0.into());
                schedule_at(Message::default().with_id(2), 2.0.into());
            },
            |_, msg| match msg.id {
                1 => {
                    // Force another event into the global queue.
                    schedule_at(Message::default(), 10.0.into());

                    let _ = current()
                        .spawner(ProcessingStack::default)
                        .node("bob", WithSimStartRequired(false));
                }
                2 => {
                    let child = current().child("bob").unwrap();
                    assert!(child.as_ref::<WithSimStartRequired>().0);

                    // check global access
                    assert!(globals().get(&"alice.bob".into()).is_some())
                }
                _ => {}
            },
        ),
    );

    Builder::seeded(123).build(sim.freeze()).run().map(|_| ())
}
