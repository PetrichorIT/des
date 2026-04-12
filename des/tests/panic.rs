use std::time::Duration;

use des::{
    Error, ErrorKind, Failure, Sim, globals,
    module::{Module, UnwindBehaviour},
    prelude::{Message, current, schedule_at},
    runtime::handlers::{AsyncHandler, ModuleFn},
    time::sleep,
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

    let mut rt = sim.seeded(123).build();
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

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let _ = rt.run();
}

struct PanicAtSimEnd;
impl Module for PanicAtSimEnd {
    fn at_sim_end(&mut self) -> Result<(), Error> {
        panic!("Oh no");
    }
}

#[serial]
#[test]
fn catch_panic_at_sim_end() {
    let mut sim = Sim::new(());
    sim.node("alice", PanicAtSimEnd);
    let gate = sim.gate("alice", "port");

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let _ = rt.run();
}

struct SimPanicAtHandle;
impl Module for SimPanicAtHandle {
    fn handle_message(&mut self, _msg: Message) {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_abort: true,
            ..Default::default()
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

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().error.unwrap();
    assert!(matches!(err[0].kind, ErrorKind::ModulePanic(_)));
}

struct SimPanicAtSimStart;
impl Module for SimPanicAtSimStart {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_abort: true,
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

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().error.unwrap();
    assert!(matches!(err[0].kind, ErrorKind::ModulePanic(_)));
}

struct SimPanicAtSimEnd;
impl Module for SimPanicAtSimEnd {
    fn at_sim_end(&mut self) -> Result<(), Error> {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_abort: true,
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

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().error.unwrap();

    assert!(matches!(err[0].kind, ErrorKind::ModulePanic(_)));
}

struct PanicWithUnwindAllways;
impl Module for PanicWithUnwindAllways {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_abort: true,
            ..UnwindBehaviour::default()
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

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    let err = rt.run().error.unwrap();
    assert!(matches!(err[0].kind, ErrorKind::ModulePanic(_)));
}

struct PanicAtRecvWithRestart;
impl Module for PanicAtRecvWithRestart {
    fn at_sim_start(&mut self, _stage: usize) {
        current().set_unwind_behaviour(UnwindBehaviour {
            on_panic_recover: true,
            ignore_panics: true,
            ..Default::default()
        });
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!();
    }
}

#[serial]
#[test]
fn unwind_and_restart() -> Result<(), Failure> {
    des::tracing::init();

    let mut sim = Sim::new(());
    sim.node("alice", PanicAtRecvWithRestart);
    sim.node(
        "bob",
        ModuleFn::new(
            || schedule_at(Message::default(), 10.0.into()),
            |_, _| {
                let alice = globals().get(&"alice").unwrap();
                assert!(alice.is_active());
            },
        ),
    );
    let gate = sim.gate("alice", "port");

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 5.0.into());
    rt.run().into_result().map(|_| ())
}

#[serial]
#[test]
fn task_panic_unobserved() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.set_default_unwind_behavior(UnwindBehaviour {
        ignore_panics: true,
        ..Default::default()
    });

    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
            });

            tokio::spawn(async move {
                sleep(Duration::from_secs(1)).await;
                panic!("tokio task paniced")
            });
        }),
    );

    let rt = sim.seeded(123).build();
    let res = rt.run();
    assert!(res.error.is_none());
    assert_eq!(res.time, 10.0);

    Ok(())
}

#[serial]
#[test]
fn task_panic_will_only_report() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            current().join(tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
            }));

            current().join(tokio::spawn(async move {
                sleep(Duration::from_secs(1)).await;
                panic!("tokio task paniced")
            }));
        }),
    );

    let rt = sim.seeded(123).build();
    let res = rt.run();
    assert!(res.error.is_some());
    assert_eq!(res.time, 10.0);

    Ok(())
}

#[serial]
#[test]
fn task_spawned_panic_will_crash() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.set_default_unwind_behavior(UnwindBehaviour {
        on_panic_abort: true,
        ..Default::default()
    });
    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            current().join(tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
            }));

            current().join(tokio::spawn(async move {
                sleep(Duration::from_secs(1)).await;
                panic!("tokio task paniced")
            }));
        }),
    );

    let rt = sim.seeded(123).build();
    let res = rt.run();
    assert!(res.error.is_some());
    assert_eq!(res.time, 1.0);

    Ok(())
}

#[serial]
#[test]
fn task_async_handler_panic_will_crash() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.set_default_unwind_behavior(UnwindBehaviour {
        on_panic_abort: true,
        ..Default::default()
    });
    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            sleep(Duration::from_secs(1)).await;
            panic!("tokio task paniced")
        }),
    );

    let rt = sim.seeded(123).build();
    let res = rt.run();
    assert!(res.error.is_some());
    assert_eq!(res.time, 1.0);

    Ok(())
}
