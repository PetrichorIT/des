use des::{
    net::{Error, handlers::AsyncHandler},
    prelude::*,
};

fn main() -> Result<(), Error> {
    let mut sim = Sim::new(());
    sim.node("alice", AsyncHandler::io(|_| async { Ok(()) }));
    sim.node("bob", B);
    sim.node("eve", B);

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

struct B;
impl Module for B {
    fn at_sim_end(&mut self) -> Result<(), Error> {
        current().set_unwind_behaviour(des::net::module::UnwindBehaviour {
            on_panic_catch: false,
            ..Default::default()
        });

        current()
            .prop::<String>("this is a funny key")?
            .set("value".into());
        current().prop::<u32>("this is a funny key")?;

        // panic!("it ends to fast {}", 1)
        Ok(())
    }
}
