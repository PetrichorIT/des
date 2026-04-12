use des::prelude::*;
use des::{Error, Failure, processing::*};
use serial_test::serial;
use std::sync::Arc;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct IncrementIncomingId;
impl ProcessingElement for IncrementIncomingId {
    fn process(&mut self, mut msg: Message) -> Option<Message> {
        msg.header.id += 1;
        Some(msg)
    }
}

#[derive(Default)]
struct PluginCreation {
    sum: usize,
}
impl Module for PluginCreation {
    fn at_sim_start(&mut self, _stage: usize) {
        for i in 0..100 {
            schedule_at(
                Message::default().with_id(i),
                SimTime::now() + Duration::from_secs(i as u64),
            )
        }
    }

    fn handle_message(&mut self, msg: Message) {
        assert_eq!(SimTime::now().as_secs() + 1, msg.header.id as u64);
        self.sum += msg.header.id as usize;
    }

    fn at_sim_end(&mut self) -> Result<(), Error> {
        assert_eq!(self.sum, (0..100).sum::<usize>() + 100);
        Ok(())
    }
}

#[test]
#[serial]
fn plugin_raw_creation() {
    let mut sim = Sim::new(());
    sim.set_stack(|| IncrementIncomingId);
    sim.node("root", PluginCreation::default());

    let rt = sim.seeded(123).build();
    let result = rt.run().assert_no_err();

    assert_eq!(result.time, SimTime::from_duration(Duration::from_secs(99)));
    assert_eq!(result.app.profiler.event_count, 101); // (+1 start signal)
}

struct ActivitySensor {
    pub expected: usize,
    pub shared: Arc<AtomicUsize>,
}
impl ProcessingElement for ActivitySensor {
    fn process_with(
        &mut self,
        msg: Option<Message>,
        inner: &mut dyn FnMut(Option<Message>) -> Option<Message>,
    ) -> Option<Message> {
        let real = self.shared.fetch_add(1, SeqCst);
        assert_eq!(real, self.expected);

        let res = inner(msg);

        let real = self.shared.fetch_sub(1, SeqCst);
        assert_eq!(real - 1, self.expected);

        res
    }
}

#[derive(Default)]
struct PluginPriorityDefer {
    arc: Arc<AtomicUsize>,
}
impl Module for PluginPriorityDefer {
    fn stack(&self, _: ProcessingStack) -> ProcessingStack {
        (
            ActivitySensor {
                shared: self.arc.clone(),
                expected: 0,
            },
            ActivitySensor {
                shared: self.arc.clone(),
                expected: 1,
            },
            ActivitySensor {
                shared: self.arc.clone(),
                expected: 2,
            },
        )
            .into()
    }

    fn at_sim_start(&mut self, _stage: usize) {
        for i in 0..100 {
            schedule_in(Message::default(), Duration::from_secs(i));
        }
    }

    fn handle_message(&mut self, _msg: Message) {}
}

#[test]
#[serial]
fn plugin_priority_defer() {
    let mut sim = Sim::new(());
    sim.node("root", PluginPriorityDefer::default());

    let rt = sim.seeded(123).build();
    let result = rt.run().assert_no_err();

    assert_eq!(result.time, 99.0);
    assert_eq!(result.app.profiler.event_count, 101); // (+1 start signal)
}

struct IncrementArcPlugin {
    arc: Arc<AtomicUsize>,
}
impl ProcessingElement for IncrementArcPlugin {
    fn process(&mut self, msg: Message) -> Option<Message> {
        self.arc.fetch_add(1, SeqCst);
        Some(msg)
    }
}

impl Drop for IncrementArcPlugin {
    fn drop(&mut self) {
        assert_eq!(self.arc.load(SeqCst), 20)
    }
}

#[derive(Default)]
struct PluginAtShutdown {
    arc: Arc<AtomicUsize>,
}
impl Module for PluginAtShutdown {
    fn stack(&self, _: ProcessingStack) -> ProcessingStack {
        IncrementArcPlugin {
            arc: self.arc.clone(),
        }
        .into()
    }

    fn at_sim_start(&mut self, _stage: usize) {
        if SimTime::now().as_secs() == 0 {
            // Schedule events at all time points 1..=20
            for i in 1..=20 {
                schedule_at(
                    Message::default(),
                    SimTime::from_duration(Duration::from_secs(i)),
                )
            }
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        if SimTime::now().as_secs() == 10 {
            // will be back online at second 11
            current().shutdow_and_restart_in(Duration::from_millis(500));
        }
    }

    fn at_sim_end(&mut self) -> Result<(), Error> {
        assert_eq!(self.arc.load(SeqCst), 20);
        Ok(())
    }
}

#[test]
#[serial]
fn plugin_shutdown_non_persistent_data() {
    let mut sim = Sim::new(());
    sim.node("root", PluginAtShutdown::default());

    let rt = sim.seeded(123).build();

    let res = rt.run();
    let _res = res.assert_no_err();
}

#[test]
#[serial]
fn custom_default_pe() {
    static DONE: AtomicBool = AtomicBool::new(false);

    struct EatAllAndSayDone;
    impl ProcessingElement for EatAllAndSayDone {
        fn process(&mut self, _: Message) -> Option<Message> {
            DONE.store(true, Ordering::SeqCst);
            None
        }
    }

    fn custom() -> ProcessingStack {
        EatAllAndSayDone.into()
    }

    struct A;
    impl Module for A {}

    let mut sim = Sim::new(());
    sim.set_stack(|| custom());
    sim.node("a", A);
    let gate = sim.gate("a", "port");

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 1.0.into());

    let _ = rt.run();
    assert!(DONE.load(Ordering::SeqCst));
}

struct AddEthInFlag;
struct EthFlag;
impl ProcessingElement for AddEthInFlag {
    fn process(&mut self, msg: Message) -> Option<Message> {
        Some(msg.with_extension(EthFlag))
    }
}

struct M {
    c: usize,
}
impl Module for M {
    fn stack(&self, mut stack: ProcessingStack) -> ProcessingStack {
        stack.append(AddEthInFlag);
        stack
    }

    fn handle_message(&mut self, msg: Message) {
        assert!(msg.extensions.has::<EthFlag>());
        self.c += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), Error> {
        assert_eq!(self.c, 1);
        Ok(())
    }
}

#[test]
#[serial]
fn add_extension_in_plugin() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node("m", M { c: 0 });
    let gate = sim.gate("m", "port");

    let mut rt = sim.seeded(123).build();
    rt.add_message_onto(gate, Message::default(), 1.0.into());

    rt.run().into_result().map(|_| ())
}

struct PEWithValue {
    value: usize,
}
impl ProcessingElement for PEWithValue {}

struct NodeWithPE;
impl Module for NodeWithPE {
    fn stack(&self, mut stack: ProcessingStack) -> ProcessingStack {
        stack.append(PEWithValue { value: 42 });
        stack
    }
}

struct NodeReadingPE;
impl Module for NodeReadingPE {
    fn at_sim_end(&mut self) -> Result<(), Error> {
        let parent = current().parent().expect("parent must exist");

        assert!(parent.try_as_ref::<NodeWithPE>().is_some());
        assert!(parent.try_as_ref::<PEWithValue>().is_some());
        assert_eq!(parent.try_as_ref::<PEWithValue>().unwrap().value, 42);

        Ok(())
    }
}

#[test]
#[serial]
fn downcast_proc_elements_from_other_node() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node("alice", NodeWithPE);
    sim.node("alice.observer", NodeReadingPE);

    let rt = sim.seeded(123).build();
    rt.run().into_result().map(|_| ())
}
