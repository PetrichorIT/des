#![cfg(feature = "net")]
use des::{net::watcher, prelude::*};
use serial_test::serial;

#[derive(Debug, Default)]
struct NameWritingModule;

impl Module for NameWritingModule {
    fn at_sim_start(&mut self, _stage: usize) {
        watcher().write("path", current().path())
    }
}

#[test]
#[serial]
fn observe_changed_variables_written_at_sim_start() {
    let mut sim = Sim::new(());
    sim.node("alice", NameWritingModule);
    sim.node("bob", NameWritingModule);
    sim.node("eve", NameWritingModule);
    sim.node("eve.evil", NameWritingModule);

    let mut sim = Builder::new().build(sim);
    sim.start();

    assert_eq!(
        sim.app.watcher("alice").read_clone::<ObjectPath>("path"),
        Some(ObjectPath::from("alice"))
    );

    assert_eq!(
        sim.app.watcher("bob").read_clone::<ObjectPath>("path"),
        Some(ObjectPath::from("bob"))
    );

    assert_eq!(
        sim.app.watcher("eve").read_clone::<ObjectPath>("path"),
        Some(ObjectPath::from("eve"))
    );

    assert_eq!(
        sim.app.watcher("eve.evil").read_clone::<ObjectPath>("path"),
        Some(ObjectPath::from("eve.evil"))
    );
}

#[derive(Debug, Default)]
struct ChangingVariableThroughEvents;

impl Module for ChangingVariableThroughEvents {
    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(Message::new().build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, msg: Message) {
        watcher().write("time", SimTime::now());
        schedule_in(msg, Duration::from_secs(1));
    }
}

#[test]
#[serial]
fn observe_changed_variables_written_in_events() {
    let mut sim = Sim::new(());
    sim.node("alice", ChangingVariableThroughEvents);

    let mut sim = Builder::new().max_itr(1000).build(sim);
    sim.start();

    assert_eq!(sim.app.watcher("alice").read_clone::<SimTime>("time"), None);

    sim.dispatch_n_events(10);

    assert_eq!(
        sim.app.watcher("alice").read_clone::<SimTime>("time"),
        Some(10.0.into())
    );

    sim.dispatch_all();

    assert_eq!(
        sim.app.watcher("alice").read_clone::<SimTime>("time"),
        Some(1000.0.into())
    );
}
