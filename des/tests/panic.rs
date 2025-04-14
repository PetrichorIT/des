use des::{
    net::{
        module::{Module, Stereotyp},
        ObjectPath, PanicError, Sim,
    },
    prelude::{current, Message},
    runtime::{Builder, RuntimeError},
};
use serial_test::serial;

struct PanicAtHandle;
impl Module for PanicAtHandle {
    fn handle_message(&mut self, _msg: Message) {
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn catch_panic_at_handle_message() {
    let mut sim = Sim::new(());
    sim.node("alice", PanicAtHandle);
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let _ = rt.run();
}

struct PanicAtSimStart;
impl Module for PanicAtSimStart {
    fn at_sim_start(&mut self, _stage: usize) {
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn catch_panic_at_sim_start() {
    let mut sim = Sim::new(());
    sim.node("alice", PanicAtSimStart);
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let _ = rt.run();
}

struct PanicAtSimEnd;
impl Module for PanicAtSimEnd {
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn catch_panic_at_sim_end() {
    let mut sim = Sim::new(());
    sim.node("alice", PanicAtSimEnd);
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let _ = rt.run();
}

struct SimPanicAtHandle;
impl Module for SimPanicAtHandle {
    fn handle_message(&mut self, _msg: Message) {
        current().set_stereotyp(Stereotyp {
            on_panic_catch: false,
            on_panic_drop: true,
            on_panic_restart: false,
            on_panic_drop_submodules: true,
            on_panic_inform_parent: false,
        });
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn unwind_sim_panic_at_handle_message() {
    let mut sim = Sim::new(());
    sim.node("alice", SimPanicAtHandle);
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();
    assert_eq!(
        err[0].as_any().downcast_ref::<PanicError>().unwrap().path,
        ObjectPath::from("alice")
    );
}

struct SimPanicAtSimStart;
impl Module for SimPanicAtSimStart {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_stereotyp(Stereotyp {
            on_panic_catch: false,
            ..Default::default()
        });
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn unwind_sim_panic_at_sim_start() {
    let mut sim = Sim::new(());
    sim.node("alice", SimPanicAtSimStart);
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();
    assert_eq!(
        err[0].as_any().downcast_ref::<PanicError>().unwrap().path,
        ObjectPath::from("alice")
    );
}

struct SimPanicAtSimEnd;
impl Module for SimPanicAtSimEnd {
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        current().set_stereotyp(Stereotyp {
            on_panic_catch: false,
            ..Default::default()
        });
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn unwind_sim_panic_at_sim_end() {
    let mut sim = Sim::new(());
    sim.node("alice", SimPanicAtSimEnd);
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();
    assert_eq!(
        err[0].as_any().downcast_ref::<PanicError>().unwrap().path,
        ObjectPath::from("alice")
    );
}

struct PanicWithUnwindAllways;
impl Module for PanicWithUnwindAllways {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_stereotyp(Stereotyp {
            on_panic_catch: false,
            ..Stereotyp::HOST
        });
    }
    fn handle_message(&mut self, _msg: Message) {
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn unwind_behaviour_unwind_allways_panics() {
    let mut sim = Sim::new(());
    sim.node("alice", PanicWithUnwindAllways);
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim);
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();
    assert_eq!(
        err[0].as_any().downcast_ref::<PanicError>().unwrap().path,
        ObjectPath::from("alice")
    );
}
