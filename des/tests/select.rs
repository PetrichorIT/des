#![cfg(feature = "macros")]
use des::prelude::*;
use std::sync::atomic::AtomicUsize;

struct Main;

static A: AtomicUsize = AtomicUsize::new(0);
static B: AtomicUsize = AtomicUsize::new(0);

impl AsyncModule for Main {
    async fn at_sim_start(&mut self, _: usize) {
        tokio::select! {
            // Note that this test may change its result, if another call to the RNG
            // is added before the simulation reaches this point.
            // Thus this test may change, however, it should only change if RNG access changes
            _ = std::future::ready(()) => {
                A.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            },
            _ = std::future::ready(()) => {
                B.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            },
        }
    }
}

#[test]
fn deterministic_branching() {
    // Since the invalid behaviour is indetermistic.,
    // check multiple iterations
    for _ in 0..100 {
        let mut rt = Sim::new(());
        rt.node("root", Main);

        let rt = Builder::seeded(123).build(rt);
        let v = rt.run();
        assert!(matches!(v, RuntimeResult::EmptySimulation { .. }));
    }

    let a = A.load(std::sync::atomic::Ordering::SeqCst);
    let b = B.load(std::sync::atomic::Ordering::SeqCst);

    assert!((a == 100 && b == 0) || (a == 0 && b == 100));
}
