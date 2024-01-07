#![cfg(feature = "net")]
use des::net::processing::*;
use des::prelude::*;
use serial_test::serial;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;

#[macro_use]
mod common;

mod lcommon {
    use des::net::processing::*;
    use des::prelude::*;

    pub struct ConsumeAllIncoming;
    impl ProcessingElement for ConsumeAllIncoming {
        fn incoming(&mut self, _msg: Message) -> Option<Message> {
            None
        }
    }

    pub struct IncrementIncomingId;
    impl ProcessingElement for IncrementIncomingId {
        fn incoming(&mut self, mut msg: Message) -> Option<Message> {
            msg.header_mut().id += 1;
            Some(msg)
        }
    }

    pub struct PanicOnIncoming;
    impl ProcessingElement for PanicOnIncoming {
        fn incoming(&mut self, _msg: Message) -> Option<Message> {
            panic!("common::PanicOnIncoming")
        }
    }
}

struct PluginCreation {
    sum: usize,
}
impl_build_named!(PluginCreation);

impl Module for PluginCreation {
    fn new() -> Self {
        Self {
            sum: 0,
        }
    }
    fn stack(&self) -> impl ProcessingElement + 'static  {
        lcommon::IncrementIncomingId
    }

    fn at_sim_start(&mut self, _stage: usize) {
        for i in 0..100 {
            schedule_at(
                Message::new().id(i).build(),
                SimTime::now() + Duration::from_secs(i as u64),
            )
        }
    }

    fn handle_message(&mut self, msg: Message) {
        assert_eq!(SimTime::now().as_secs() + 1 , msg.header().id as u64);
        self.sum += msg.header().id as usize;
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.sum, (0..100).sum::<usize>() + 100);
    }
}

#[test]
#[serial]
fn plugin_raw_creation() {
    // Logger::new().set_logger();

    let mut app = NetworkApplication::new(());

    let root = PluginCreation::build_named(ObjectPath::from("root"), &mut app);
    app.register_module(root);

    let rt = Builder::seeded(123).build(app);
    let result = rt.run().unwrap();

    assert_eq!(result.1, SimTime::from_duration(Duration::from_secs(99)));
    assert_eq!(result.2.event_count, 100);
}



struct ActivitySensor {
    pub expected: usize,
    pub shared: Arc<AtomicUsize>,
}
impl ProcessingElement for ActivitySensor {
    fn event_start(&mut self) {
        let real = self.shared.fetch_add(1, SeqCst);
        assert_eq!(real, self.expected);
    }

    fn event_end(&mut self) {
        let real = self.shared.fetch_sub(1, SeqCst);
        assert_eq!(real - 1, self.expected);
    }
}

struct PluginPriorityDefer {
    arc: Arc<AtomicUsize>,
}
impl_build_named!(PluginPriorityDefer);
impl Module for PluginPriorityDefer {
    fn new() -> Self {
        Self {
            arc: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn stack(&self) -> impl IntoProcessingElements {
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
            }
        )
    }

    fn at_sim_start(&mut self, _stage: usize) {
        for i in 0..100 {
            schedule_in(Message::new().build(), Duration::from_secs(i));
        }
    }

    fn handle_message(&mut self, _msg: Message) {}
}

#[test]
#[serial]
fn plugin_priority_defer() {
    // Logger::new().set_logger();

    let mut app = NetworkApplication::new(());

    let module = PluginPriorityDefer::build_named(ObjectPath::from("root"), &mut app);
    app.register_module(module);

    let rt = Builder::seeded(123).build(app);
    let result = rt.run();

    let RuntimeResult::Finished { time, profiler, .. } = result else {
        panic!("Unexpected runtime result")
    };

    assert_eq!(time, 99.0);
    assert_eq!(profiler.event_count, 100);
}

struct IncrementArcPlugin {
    arc: Arc<AtomicUsize>,
}
impl ProcessingElement for IncrementArcPlugin {
    fn incoming(&mut self, msg: Message) -> Option<Message> {
        self.arc.fetch_add(1, SeqCst);
        Some(msg)
    }
}

impl Drop for IncrementArcPlugin {
    fn drop(&mut self) {
        assert_eq!(self.arc.load(SeqCst), 20)
    }
}

struct PluginAtShutdown {
    arc: Arc<AtomicUsize>,
}
impl_build_named!(PluginAtShutdown);
impl Module for PluginAtShutdown {
    fn new() -> Self {
        Self {
            arc: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn stack(&self) -> impl IntoProcessingElements {
        IncrementArcPlugin { arc: self.arc.clone() }
    }


    fn at_sim_start(&mut self, _stage: usize) {
        if SimTime::now().as_secs() == 0 {
            // Schedule events at all time points 1..=20
            for i in 1..=20 {
                schedule_at(
                    Message::new().build(),
                    SimTime::from_duration(Duration::from_secs(i)),
                )
            }
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        if SimTime::now().as_secs() == 10 {
            // will be back online at second 11
            shutdow_and_restart_in(Duration::from_millis(500));
        }
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.arc.load(SeqCst), 20);
    }
}

#[test]
#[serial]
fn plugin_shutdown_non_persistent_data() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = NetworkApplication::new(());

    let module = PluginAtShutdown::build_named(ObjectPath::from("root".to_string()), &mut rt);
    rt.register_module(module);

    let rt = Builder::seeded(123).build(rt);

    let res = rt.run();
    let _res = res.unwrap();
}