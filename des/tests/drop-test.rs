#![cfg(feature = "net")]

use std::{
    error::Error,
    ops::Deref,
    sync::{atomic::AtomicUsize, Arc},
};

use des::{
    net::ndl::Registry,
    net::{blocks::ModuleBlock, module::Module, Sim},
    runtime::Builder,
};
use serial_test::serial;

struct Harness<A>(pub A);
impl<A> Deref for Harness<A> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[test]
#[serial]
fn drop_check_modules_net_from_sim_builder() {
    let drop_counter = Arc::new(AtomicUsize::new(0));
    let mut sim = Sim::new(());
    sim.node(
        "b",
        B {
            counter: drop_counter.clone(),
        },
    );
    sim.node(
        "a",
        Harness(A {
            counter: drop_counter.clone(),
        }),
    );

    let rtr = Builder::seeded(123).build(sim.freeze()).run();
    drop(rtr);

    assert_eq!(drop_counter.load(std::sync::atomic::Ordering::SeqCst), 3);
}

#[test]
#[serial]
fn drop_check_modules_net_from_ndl() -> Result<(), Box<dyn Error>> {
    let drop_counter = Arc::new(AtomicUsize::new(0));
    let registry = Registry::new()
        .symbol_fn("Alice", |_| B {
            counter: drop_counter.clone(),
        })
        .symbol_fn("Bob", |_| B {
            counter: drop_counter.clone(),
        })
        .with_default_fallback();

    let sim = Sim::ndl("tests/ndl/drop-test.yml", registry)?;
    let rtr = Builder::seeded(123).build(sim.freeze()).run();
    drop(rtr);

    assert_eq!(drop_counter.load(std::sync::atomic::Ordering::SeqCst), 2);
    Ok(())
}

struct A {
    counter: Arc<AtomicUsize>,
}

impl ModuleBlock for Harness<A> {
    type Ret = ();
    fn build<A>(self, mut sim: des::prelude::SimBuilderScoped<'_, A>) {
        let counter = self.counter.clone();
        sim.root(self.0);
        sim.node("b", B { counter });
    }
}

impl Module for A {}

struct B {
    counter: Arc<AtomicUsize>,
}

impl Drop for A {
    fn drop(&mut self) {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Module for B {}

impl Drop for B {
    fn drop(&mut self) {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}
