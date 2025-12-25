use des::{
    net::{
        Error, globals,
        handlers::ModuleFn,
        message::Body,
        module::{SIGNAL_MODULE_PANICED, Signal, UnwindBehaviour, emit},
        processing::ProcessingStack,
    },
    prelude::*,
};
use serial_test::serial;

mod common;
use common::*;

struct Parent(bool);
impl Module for Parent {
    fn at_sim_start(&mut self, _stage: usize) {
        current().subscribe_to(SIGNAL_MODULE_PANICED);
    }
    fn handle_signal(&mut self, signal: Signal) {
        match signal.code {
            SIGNAL_MODULE_PANICED => {
                // Handle the panic signal
                self.0 = true;
            }
            _ => {}
        }
    }
    fn at_sim_end(&mut self) -> Result<(), Error> {
        assert!(self.0, "Must have observed child's panic");
        Ok(())
    }
}

fn panicing_subprocess_at(t: impl Into<SimTime>) -> impl Module {
    let t = t.into();
    ModuleFn::new(
        move || {
            current().set_unwind_behaviour(UnwindBehaviour::SUBPROCESS);
            schedule_at(Message::default(), t)
        },
        |_, _| panic!("some reason"),
    )
}

#[test]
#[serial]
fn signal_subscription_in_direct_parent() -> Result<(), Error> {
    let mut sim = Sim::new(());
    sim.node("parent", Parent(false));
    sim.node("parent.child", panicing_subprocess_at(2.0));

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

#[test]
#[serial]
fn signal_subscription_in_indirect_ancestor() -> Result<(), Error> {
    let mut sim = Sim::new(());
    sim.node("parent", Parent(false));
    sim.node("parent.child", NopModule);
    sim.node("parent.child.grandchild", NopModule);
    sim.node(
        "parent.child.grandchild.greatgrandchild",
        panicing_subprocess_at(2.0),
    );

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

#[test]
#[serial]
fn signal_subscription_passed_to_created_child() -> Result<(), Error> {
    let mut sim = Sim::new(());
    sim.node("parent", Parent(false));
    sim.node(
        "other",
        ModuleFn::new(
            || schedule_at(Message::default(), 2.0.into()),
            |_, _| {
                println!("[{}] 1 {}", SimTime::now(), current().path());

                let g = globals();
                println!("2");
                let p = g.get(&"parent").unwrap();
                println!("3");
                p.spawner(ProcessingStack::default)
                    .node("child", panicing_subprocess_at(4.0));

                println!("[{}] 2 {}", SimTime::now(), current().path());
            },
        ),
    );

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

struct ExpectNSignal<const SIGNAL: usize>(i32);
impl<const SIGNAL: usize> Module for ExpectNSignal<SIGNAL> {
    fn at_sim_start(&mut self, _stage: usize) {
        current().subscribe_to(SIGNAL);
    }
    fn handle_signal(&mut self, signal: des::net::module::Signal) {
        if signal.code == SIGNAL {
            self.0 -= 1;
        }
    }
    fn at_sim_end(&mut self) -> Result<(), Error> {
        assert_eq!(self.0, 0, "expected {} more messages", self.0);
        Ok(())
    }
}

struct EmitSignal<const SIGNAL: usize>(usize);
impl<const SIGNAL: usize> Module for EmitSignal<SIGNAL> {
    fn at_sim_start(&mut self, _stage: usize) {
        assert!(self.0 > 0);
        schedule_in(Message::default(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, _msg: Message) {
        emit(SIGNAL, Body::empty());
        self.0 -= 1;
        if self.0 > 0 {
            schedule_in(Message::default(), Duration::from_secs(1));
        }
    }
}

#[test]
#[serial]
fn signal_subscription_from_multiple_children() -> Result<(), Error> {
    const SIGNAL: usize = 32;

    let mut sim = Sim::new(());
    sim.node("parent", ExpectNSignal::<SIGNAL>(10));
    sim.node("parent.child", EmitSignal::<SIGNAL>(3));
    sim.node("parent.child.grandchild", EmitSignal::<SIGNAL>(7));

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}

struct ExpectNSignalThenUnsubscribe<const SIGNAL: usize>(i32);
impl<const SIGNAL: usize> Module for ExpectNSignalThenUnsubscribe<SIGNAL> {
    fn at_sim_start(&mut self, _stage: usize) {
        current().subscribe_to(SIGNAL);
    }
    fn handle_signal(&mut self, signal: des::net::module::Signal) {
        if signal.code == SIGNAL {
            self.0 -= 1;
            if self.0 == 0 {
                current().unsubscribe_from(SIGNAL);
            }
        }
    }
    fn at_sim_end(&mut self) -> Result<(), Error> {
        assert_eq!(self.0, 0, "expected {} more messages", self.0);
        Ok(())
    }
}

#[test]
#[serial]
fn signal_unsubscribe() -> Result<(), Error> {
    const SIGNAL: usize = 32;

    let mut sim = Sim::new(());
    sim.node("parent", ExpectNSignalThenUnsubscribe::<SIGNAL>(10));
    sim.node("parent.child", EmitSignal::<SIGNAL>(3));
    sim.node("parent.child.grandchild", EmitSignal::<SIGNAL>(17));

    sim.node(
        "parent.observer-child",
        ModuleFn::new(
            || {
                schedule_at(Message::default().with_id(1), 1.0.into());
                schedule_at(Message::default().with_id(2), 10.0.into());
            },
            |_, msg| match msg.id {
                1 => assert!(current().has_subscribers(SIGNAL)),
                2 => assert!(!current().has_subscribers(SIGNAL)),
                _ => {}
            },
        ),
    );

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}
