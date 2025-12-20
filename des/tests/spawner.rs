use des::{
    net::{
        globals,
        handlers::{ModuleFn, WithContext},
        processing::ProcessingStack,
    },
    prelude::*,
};
use serial_test::serial;

struct WithSimStartRequired(bool);
impl Module for WithSimStartRequired {
    fn at_sim_start(&mut self, _stage: usize) {
        self.0 = true;
    }
    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert!(self.0, "must be set by at_sim_start");
        Ok(())
    }
}

#[test]
#[serial]
fn runtime_spawner_calls_sim_start() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::new(
            || {
                schedule_at(Message::default().with_id(1), 1.0.into());
                schedule_at(Message::default().with_id(2), 2.0.into());
            },
            |_, msg| match msg.id {
                1 => {
                    // Force another event into the global queue.
                    schedule_at(Message::default(), 10.0.into());

                    let _ = current()
                        .spawner(ProcessingStack::default)
                        .node("bob", WithSimStartRequired(false));
                }
                2 => {
                    let child = current().child("bob").unwrap();
                    assert!(child.as_ref::<WithSimStartRequired>().0);

                    // check global access
                    assert!(globals().get(&"alice.bob".into()).is_some())
                }
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

#[test]
#[serial]
fn runtime_spawner_with_mod_ctx() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::new(
            || {
                schedule_at(Message::default().with_id(1), 1.0.into());
                schedule_at(Message::default().with_id(2), 2.0.into());
            },
            |_, msg| match msg.id {
                1 => {
                    // Force another event into the global queue.
                    schedule_at(Message::default(), 10.0.into());

                    assert_eq!(current().path().as_str(), "alice");
                    let _ = current().spawner(ProcessingStack::default).node(
                        "bob",
                        WithContext(|| {
                            assert_eq!(current().path().as_str(), "alice.bob");
                            WithSimStartRequired(false)
                        }),
                    );
                    assert_eq!(current().path().as_str(), "alice");
                }
                2 => {
                    let child = current().child("bob").unwrap();
                    assert!(child.as_ref::<WithSimStartRequired>().0);

                    // check global access
                    assert!(globals().get(&"alice.bob".into()).is_some())
                }
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

#[test]
#[serial]
fn runtime_spawner_cannot_use_root() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        ModuleFn::new(
            || {
                schedule_at(Message::default().with_id(1), 1.0.into());
            },
            |_, msg| match msg.id {
                1 => {
                    // Force another event into the global queue.
                    schedule_at(Message::default(), 10.0.into());

                    let _ = current()
                        .spawner(ProcessingStack::default)
                        .root_with_context(|| {
                            assert_eq!(current().path().as_str(), "alice");
                            Box::new(WithSimStartRequired(false))
                        });
                }
                _ => {}
            },
        ),
    );

    let _ = Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .error
        .expect("must have failed");
    Ok(())
}

#[test]
#[serial]
fn runtime_spawner_reads_cfgs() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.include_cfg("alice.bob.key: 123");
    sim.node(
        "alice",
        ModuleFn::new(
            || {
                schedule_at(Message::default().with_id(1), 1.0.into());
            },
            |_, msg| match msg.id {
                1 => {
                    let _ = current().spawner(ProcessingStack::default).node(
                        "bob",
                        WithContext(|| {
                            assert_eq!(
                                current().prop::<u32>("key").unwrap().or_default().get(),
                                123
                            );
                            WithSimStartRequired(false)
                        }),
                    );
                }
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

struct ProcElementWithSubmodule {
    handle: Option<ModuleRef>,
}
impl ProcessingElement for ProcElementWithSubmodule {
    fn process(&mut self, msg: Message) -> Option<Message> {
        let handle = match self.handle.take() {
            Some(handle) => handle,
            None => match current().child("sub-proc-element") {
                Ok(handle) => handle,
                Err(_) => {
                    current()
                        .spawner(ProcessingStack::default)
                        .node("sub-proc-element", WithSimStartRequired(true));
                    return Some(msg); // must stop now, since not yet started
                }
            },
        };

        let value = handle.as_ref::<WithSimStartRequired>().0;
        self.handle = Some(handle);
        Some(msg.with_extension(value))
    }
}

struct MyProcElementModule {
    c: usize,
}
impl Module for MyProcElementModule {
    fn stack(&self, mut stack: ProcessingStack) -> ProcessingStack {
        stack.append(ProcElementWithSubmodule { handle: None });
        stack
    }

    fn at_sim_start(&mut self, _stage: usize) {
        schedule_at(Message::default().with_id(1), 1.0.into());
        schedule_at(Message::default().with_id(2), 2.0.into());
        schedule_at(Message::default().with_id(3), 3.0.into());
    }

    fn handle_message(&mut self, msg: Message) {
        let is_first = msg.id == 1;
        assert!(is_first || msg.extensions.has::<bool>());
        self.c += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.c, 3);
        Ok(())
    }
}

#[test]
#[serial]
fn runtime_spawner_from_proc_element() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());
    sim.include_cfg("alice.bob.key: 123");
    sim.node("alice", MyProcElementModule { c: 0 });

    Builder::seeded(123)
        .build(sim.freeze())
        .run()
        .as_result()
        .map(|_| ())
}
