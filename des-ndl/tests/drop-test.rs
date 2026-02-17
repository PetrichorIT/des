use std::{
    error::Error,
    sync::{Arc, atomic::AtomicUsize},
};

use des::{
    net::{Sim, module::Module},
    runtime::Builder,
};
use des_ndl::{Ndl, Registry};
use serial_test::serial;

#[test]
#[serial]
fn drop_check_modules_net_from_ndl() -> Result<(), Box<dyn Error>> {
    let drop_counter = Arc::new(AtomicUsize::new(0));
    let mut registry = Registry::new()
        .symbol_fn("Alice", |_| B {
            counter: drop_counter.clone(),
        })
        .symbol_fn("Bob", |_| B {
            counter: drop_counter.clone(),
        })
        .with_default_fallback();

    let mut sim = Sim::new(());
    sim.node(
        "",
        Ndl::from_str(&mut registry, include_str!("ndl/drop-test.yml"))?,
    )?;
    let rtr = Builder::seeded(123).build(sim.freeze()).run();
    drop(rtr);

    assert_eq!(drop_counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    Ok(())
}

struct B {
    counter: Arc<AtomicUsize>,
}
impl Module for B {}

impl Drop for B {
    fn drop(&mut self) {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
