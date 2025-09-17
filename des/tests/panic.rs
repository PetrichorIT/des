use des::{
    net::{
        Error, ErrorKind, Sim, globals,
        handlers::ModuleFn,
        module::{Module, UnwindBehaviour},
    },
    prelude::{Message, current, schedule_at},
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

    let mut rt = Builder::seeded(123).build(sim.freeze());
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

    let mut rt = Builder::seeded(123).build(sim.freeze());
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

    let mut rt = Builder::seeded(123).build(sim.freeze());
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let _ = rt.run();
}

struct SimPanicAtHandle;
impl Module for SimPanicAtHandle {
    fn handle_message(&mut self, _msg: Message) {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_catch: false,
            on_panic_restart: false,
            on_panic_drop_submodules: true,
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

    let mut rt = Builder::seeded(123).build(sim.freeze());
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();
    assert!(matches!(
        err[0].as_any().downcast_ref::<Error>().unwrap().kind,
        ErrorKind::ModulePanic(_)
    ));
}

struct SimPanicAtSimStart;
impl Module for SimPanicAtSimStart {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_unwind_behaviour(UnwindBehaviour {
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

    let mut rt = Builder::seeded(123).build(sim.freeze());
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();
    assert!(matches!(
        err[0].as_any().downcast_ref::<Error>().unwrap().kind,
        ErrorKind::ModulePanic(_)
    ));
}

struct SimPanicAtSimEnd;
impl Module for SimPanicAtSimEnd {
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        current().set_unwind_behaviour(UnwindBehaviour {
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

    let mut rt = Builder::seeded(123).build(sim.freeze());
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();

    assert!(matches!(
        err[0].as_any().downcast_ref::<Error>().unwrap().kind,
        ErrorKind::ModulePanic(_)
    ));
}

struct PanicWithUnwindAllways;
impl Module for PanicWithUnwindAllways {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_catch: false,
            ..UnwindBehaviour::HOST
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

    let mut rt = Builder::seeded(123).build(sim.freeze());
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().unwrap_err();
    assert!(matches!(
        err[0].as_any().downcast_ref::<Error>().unwrap().kind,
        ErrorKind::ModulePanic(_)
    ));
}

struct PanicAtRecvWithRestart;
impl Module for PanicAtRecvWithRestart {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_catch: true,
            on_panic_restart: true,
            on_panic_drop_submodules: false,
        });
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!();
    }
}

#[serial]
#[test]
fn unwind_and_restart() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node("alice", PanicAtRecvWithRestart);
    sim.node(
        "bob",
        ModuleFn::new(
            || schedule_at(Message::default(), 10.0.into()),
            |_, _| {
                let alice = globals().get(&"alice".into()).unwrap();
                assert!(alice.is_active());
            },
        ),
    );
    let gate = sim.gate("alice", "port");

    let mut rt = Builder::seeded(123).build(sim.freeze());
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    rt.run().map(|_| ())
}
