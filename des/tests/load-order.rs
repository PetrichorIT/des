use des::prelude::*;
use des::runtime::handlers::ModuleFn;
use std::sync::{Arc, atomic::AtomicU16};

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

    let _ = sim.seeded(123).build().run();
}
