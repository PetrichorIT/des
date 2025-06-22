use des::{net::blocks::AsyncFn, prelude::*};
use std::panic;

fn main() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node("alice", AsyncFn::io(|_| async { Ok(()) }));
    sim.node("bob", B);
    sim.node("eve", B);

    Builder::seeded(123).build(sim.freeze()).run().map(|_| ())
}

struct B;
impl Module for B {
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        current().set_stereotyp(des::net::module::Stereotyp {
            on_panic_catch: false,
            ..Default::default()
        });
        panic!("it ends to fast {}", 1)
    }
}
