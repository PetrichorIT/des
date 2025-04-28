#![cfg(feature = "net")]

use des::net::blocks::ModuleFn;
use des::prelude::*;
use std::sync::{atomic::AtomicU16, Arc};

#[test]
fn load_order() {
    let state = Arc::new(AtomicU16::new(0));
    let mut sim = Sim::new(());

    macro_rules! stage {
        ($i:ident == $l:literal) => {{
            let old = $i.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            assert_eq!(old, $l);
        }};
    }

    let s2 = state.clone();
    sim.node("alice", ModuleFn::new(move || stage!(s2 == 0), |_, _| {}));
    let s2 = state.clone();
    sim.node(
        "alice.submodule",
        ModuleFn::new(move || stage!(s2 == 1), |_, _| {}),
    );
    let s2 = state.clone();
    sim.node(
        "alice.bob",
        ModuleFn::new(move || stage!(s2 == 3), |_, _| {}),
    );
    let s2 = state.clone();
    sim.node(
        "alice.submodule.sub",
        ModuleFn::new(move || stage!(s2 == 2), |_, _| {}),
    );

    let _ = Builder::seeded(123).build(sim).run();
}
