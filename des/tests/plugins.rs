#![cfg(feature = "net")]
use des::net::{plugin2::*, BuildContext, __Buildable0};
use des::prelude::*;
use serial_test::serial;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;

mod common {
    use des::net::plugin2::*;
    use des::prelude::*;

    pub struct ConsumeAllIncoming;
    impl Plugin for ConsumeAllIncoming {
        fn capture_incoming(&mut self, _msg: Message) -> Option<Message> {
            None
        }
    }

    pub struct IncrementIncomingId;
    impl Plugin for IncrementIncomingId {
        fn capture_incoming(&mut self, mut msg: Message) -> Option<Message> {
            msg.header_mut().id += 1;
            Some(msg)
        }
    }

    pub struct PanicOnIncoming;
    impl Plugin for PanicOnIncoming {
        fn capture_incoming(&mut self, _msg: Message) -> Option<Message> {
            panic!("common::PanicOnIncoming")
        }
    }
}

#[NdlModule]
struct PluginCreation {
    handles: Vec<PluginHandle>,
    sum: usize,
}
impl Module for PluginCreation {
    fn new() -> Self {
        Self {
            handles: Vec::new(),
            sum: 0,
        }
    }
    fn at_sim_start(&mut self, _stage: usize) {
        self.handles
            .push(add_plugin(common::IncrementIncomingId, 100));
        assert_eq!(self.handles[0].status(), PluginStatus::Active);
        for i in 0..100 {
            schedule_at(
                Message::new().id(i).build(),
                SimTime::now() + Duration::from_secs(i as u64),
            )
        }
    }
    fn handle_message(&mut self, msg: Message) {
        assert_eq!(SimTime::now().as_secs() + 1, msg.header().id as u64);
        self.sum += msg.header().id as usize;
    }
    fn at_sim_end(&mut self) {
        assert_eq!(self.sum, (0..100).sum::<usize>() + 100);
        assert_eq!(self.handles[0].status(), PluginStatus::Active);
    }
}

#[test]
#[serial]
fn plugin_creation() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let root = PluginCreation::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(root);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run().unwrap();

    assert_eq!(result.1, SimTime::from_duration(Duration::from_secs(99)));
    assert_eq!(result.2.event_count, 101);
}

struct RecrusivePluginCreationPlugin {
    level: u16,
}
impl Plugin for RecrusivePluginCreationPlugin {
    fn capture_incoming(&mut self, mut msg: Message) -> Option<Message> {
        if msg.header().id == self.level {
            add_plugin(
                Self {
                    level: self.level + 1,
                },
                self.level as usize + 1,
            );
        }
        msg.header_mut().kind += 1;
        Some(msg)
    }
}

#[NdlModule]
struct PluginInPluginCreation;
impl Module for PluginInPluginCreation {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(RecrusivePluginCreationPlugin { level: 1 }, 1);
        for i in 1..10 {
            schedule_in(
                Message::new().id(i).kind(0).build(),
                Duration::from_secs(i as u64),
            )
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let id = msg.header().id + 1; // number of modules that are active
        assert_eq!(msg.header().kind, id,);
    }
}

#[test]
#[serial]
fn plugin_in_plugin_creation() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PluginInPluginCreation::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let RuntimeResult::Finished { time, profiler, .. } = result else {
        panic!("Unexpected runtime result")
    };

    assert_eq!(time, 9.0);
    assert_eq!(profiler.event_count, 9 + 1);
}

struct RecrusivePluginCreationPlugin2 {
    level: u16,
}
impl Plugin for RecrusivePluginCreationPlugin2 {
    fn capture_incoming(&mut self, mut msg: Message) -> Option<Message> {
        if msg.header().id == self.level {
            log::info!("new subplugin");
            add_plugin(
                Self {
                    level: self.level - 1,
                },
                self.level as usize - 1,
            );
        }
        msg.header_mut().kind += 1;
        log::info!("inc");
        Some(msg)
    }
}

#[NdlModule]
struct PluginInPluginCreation2;
impl Module for PluginInPluginCreation2 {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(RecrusivePluginCreationPlugin2 { level: 10 }, 10);
        for i in 0..=10 {
            schedule_in(
                Message::new().id(10 - i).kind(0).build(),
                Duration::from_secs(i as u64),
            )
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let id = 10 - msg.header().id + 1; // number of modules that are active -1 (since one is defered)
        assert_eq!(msg.header().kind, id);
    }
}

#[test]
#[serial]
fn plugin_in_plugin_creation2() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PluginInPluginCreation2::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let RuntimeResult::Finished { time, profiler, .. } = result else {
        panic!("Unexpected runtime result")
    };

    assert_eq!(time, 10.0);
    assert_eq!(profiler.event_count, 11 + 1);
}

#[NdlModule]
struct PluginPriority;
impl Module for PluginPriority {
    fn new() -> Self {
        Self
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(common::PanicOnIncoming, 100);
        add_plugin(common::ConsumeAllIncoming, 10);

        for i in 0..100 {
            schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64));
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!("Panic on message plugin let through message")
    }
}

#[test]
#[serial]
fn plugin_priority() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PluginPriority::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let RuntimeResult::Finished { time, profiler, .. } = result else {
        panic!("Unexpected runtime result")
    };

    assert_eq!(time, 99.0);
    assert_eq!(profiler.event_count, 100 + 1);
}

struct ActivitySensor {
    pub expected: usize,
    pub shared: Arc<AtomicUsize>,
}
impl Plugin for ActivitySensor {
    fn event_start(&mut self) {
        let real = self.shared.fetch_add(1, SeqCst);
        assert_eq!(real, self.expected);
    }

    fn event_end(&mut self) {
        let real = self.shared.fetch_sub(1, SeqCst);
        assert_eq!(real - 1, self.expected);
    }
}

#[NdlModule]
struct PluginPriorityDefer {
    arc: Arc<AtomicUsize>,
}
impl Module for PluginPriorityDefer {
    fn new() -> Self {
        Self {
            arc: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(
            ActivitySensor {
                shared: self.arc.clone(),
                expected: 1,
            },
            100,
        );
        add_plugin(
            ActivitySensor {
                shared: self.arc.clone(),
                expected: 0,
            },
            10,
        );
        add_plugin(
            ActivitySensor {
                shared: self.arc.clone(),
                expected: 2,
            },
            1000,
        );

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

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PluginPriorityDefer::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let RuntimeResult::Finished { time, profiler, .. } = result else {
        panic!("Unexpected runtime result")
    };

    assert_eq!(time, 99.0);
    assert_eq!(profiler.event_count, 100 + 1);
}

#[NdlModule]
struct PluginDuplication {
    counter: usize,
}
impl Module for PluginDuplication {
    fn new() -> Self {
        Self { counter: 0 }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(common::IncrementIncomingId, 100);
        add_plugin(common::IncrementIncomingId, 1000);

        for i in 0..100 {
            schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64));
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let id = msg.header().id as usize;
        assert_eq!(id, self.counter + 2);
        self.counter += 1;
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.counter, 100)
    }
}

#[test]
#[serial]
fn plugin_duplication() {
    // Logger::new().finish().unwrap();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PluginDuplication::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let RuntimeResult::Finished { time, profiler, .. } = result else {
        panic!("Unexpected runtime result")
    };

    assert_eq!(time, 99.0);
    assert_eq!(profiler.event_count, 100 + 1);
}

#[NdlModule]
struct PluginRemoval {
    counter: usize,
    handle: Option<PluginHandle>,
}
impl Module for PluginRemoval {
    fn new() -> Self {
        Self {
            counter: 0,
            handle: None,
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        self.handle = Some(add_plugin(common::IncrementIncomingId, 100));
        add_plugin(common::IncrementIncomingId, 1000);

        for i in 0..100 {
            schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64));
        }

        schedule_in(Message::new().kind(42).build(), Duration::from_secs(123));

        for i in 0..100 {
            schedule_in(
                Message::new().id(200 + i).build(),
                Duration::from_secs(200 + i as u64),
            );
        }
    }

    fn handle_message(&mut self, msg: Message) {
        if msg.header().kind == 42 {
            assert_eq!(self.counter, 100);
            self.handle.take().unwrap().remove();
            self.counter = 199;
            return;
        }

        let id = msg.header().id as usize;
        assert_eq!(id, self.counter + 2);
        self.counter += 1;
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.counter, 299);
    }
}

#[test]
#[serial]
fn plugin_removal() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PluginRemoval::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let RuntimeResult::Finished { time, profiler, .. } = result else {
        panic!("Unexpected runtime result")
    };

    assert_eq!(time, 299.0);
    assert_eq!(profiler.event_count, 201 + 1);
}

#[NdlModule]
struct PanicPolicyAbort;
impl Module for PanicPolicyAbort {
    fn new() -> Self {
        Self
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin_with(common::PanicOnIncoming, 100, PluginPanicPolicy::Abort);
        for i in 0..10 {
            schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64))
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!("Should never reach this point");
    }
}

#[test]
#[serial]
#[should_panic = "common::PanicOnIncoming"]
fn plugin_panic_abort() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PanicPolicyAbort::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let _result = rt.run();

    panic!("Should never have reached this point")
}

#[NdlModule]
struct PanicPolicyCapture;
impl Module for PanicPolicyCapture {
    fn new() -> Self {
        Self
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin_with(common::PanicOnIncoming, 100, PluginPanicPolicy::Capture);
        for i in 0..10 {
            schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64))
        }
    }

    fn handle_message(&mut self, msg: Message) {
        assert!(msg.header().id > 0)
    }
}

#[test]
#[serial]
fn plugin_panic_capture() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PanicPolicyCapture::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let result = result.unwrap();
    assert_eq!(result.1.as_secs(), 9);
    assert_eq!(result.2.event_count, 10 + 1);
}

struct PanicAtThree;
impl Plugin for PanicAtThree {
    fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
        if msg.header().id == 3 {
            panic!("I dont like this number")
        }
        Some(msg)
    }
}

#[NdlModule]
struct PanicPolicyRestart {
    handle: Vec<PluginHandle>,
}
impl Module for PanicPolicyRestart {
    fn new() -> Self {
        Self { handle: Vec::new() }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        let h = add_plugin_with(
            PanicAtThree,
            100,
            PluginPanicPolicy::Restart(Arc::new(|| Box::new(PanicAtThree))),
        );
        self.handle.push(h);

        for i in 0..100 {
            schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64))
        }
    }

    fn handle_message(&mut self, msg: Message) {
        assert_ne!(msg.header().id, 3);
        assert_eq!(msg.header().id, SimTime::now().as_secs() as u16);

        assert_eq!(self.handle[0].status(), PluginStatus::Active);
    }
}

#[test]
#[serial]
fn plugin_panic_restart() {
    // Logger::new().set_logger();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = PanicPolicyRestart::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
    let result = rt.run();

    let result = result.unwrap();
    assert_eq!(result.1.as_secs(), 99);
    assert_eq!(result.2.event_count, 100 + 1);
}

// #[NdlModule]
// struct PluginAtShutdown {
//     state: Arc<AtomicUsize>,
//     restarted: bool,
// }
// impl Module for PluginAtShutdown {
//     fn new() -> Self {
//         Self {
//             state: Arc::new(AtomicUsize::new(0)),
//             restarted: false,
//         }
//     }

//     fn reset(&mut self) {
//         self.restarted = true
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         add_plugin(
//             PeriodicPlugin::new(
//                 |state| {
//                     state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
//                     if SimTime::now().as_secs() == 5 {
//                         shutdow_and_restart_in(Duration::from_secs(2));
//                     }
//                 },
//                 Duration::from_secs(1),
//                 self.state.clone(),
//             ),
//             0,
//         );
//     }

//     fn handle_message(&mut self, _: Message) {
//         panic!("This function should never be called")
//     }

//     fn at_sim_end(&mut self) {
//         // at 1,2,3,4,5 .. 8,9,10
//         assert!(self.restarted);
//         assert_eq!(self.state.load(std::sync::atomic::Ordering::SeqCst), 5 + 3)
//     }
// }

// #[test]
// #[serial]
// fn plugin_at_shutdown() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut app = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut app);

//     let module = PluginAtShutdown::build_named(ObjectPath::root_module("root"), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(
//         app,
//         RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(10))),
//     );
//     let result = rt.run();

//     let RuntimeResult::PrematureAbort { time, profiler, .. } = result else {
//         panic!("Unexpected runtime result")
//     };

//     assert_eq!(time, 10.0);
//     assert_eq!(profiler.event_count, 12);
// }

// #[NdlModule]
// struct PeriodicMultiModule {
//     state: Arc<AtomicUsize>,
// }

// impl Module for PeriodicMultiModule {
//     fn new() -> Self {
//         PeriodicMultiModule {
//             state: Arc::new(AtomicUsize::new(0)),
//         }
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         add_plugin(
//             PeriodicPlugin::new(
//                 |state| {
//                     state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
//                 },
//                 Duration::from_secs(1),
//                 self.state.clone(),
//             ),
//             0,
//         );
//         add_plugin(
//             PeriodicPlugin::new(
//                 |state| {
//                     state.fetch_add(2, std::sync::atomic::Ordering::SeqCst);
//                 },
//                 Duration::from_secs(2),
//                 self.state.clone(),
//             ),
//             0,
//         );
//     }

//     fn handle_message(&mut self, _msg: Message) {
//         panic!("This function should never be called")
//     }

//     fn at_sim_end(&mut self) {
//         assert_eq!(self.state.load(std::sync::atomic::Ordering::SeqCst), 20)
//     }
// }

// #[test]
// #[serial]
// fn plugin_periodic_plugin() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut rt = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut rt);

//     let module =
//         PeriodicMultiModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(
//         rt,
//         RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(10))),
//     );

//     let res = dbg!(rt.run());
//     let res = res.unwrap_premature_abort();
//     assert_eq!(res.3, 2);
// }

// struct PluginErrorPlugin(Arc<AtomicBool>);
// impl Plugin for PluginErrorPlugin {
//     fn capture(&mut self, msg: Option<Message>) -> Option<Message> {
//         if SimTime::now().as_secs() > 20 {
//             panic!("Test-Panic to get plugin error")
//         }
//         self.0.store(true, SeqCst);
//         msg
//     }

//     fn defer(&mut self) {
//         self.0.store(false, SeqCst)
//     }
// }

// #[NdlModule]
// struct PluginErrorModule {
//     flag: Arc<AtomicBool>,
//     done: bool,
// }
// impl Module for PluginErrorModule {
//     fn new() -> Self {
//         Self {
//             flag: Arc::new(AtomicBool::new(false)),
//             done: false,
//         }
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         let err = PluginError::expected::<PluginErrorPlugin>();
//         assert_eq!(
//             format!("{err}"),
//             "expected plugin of type plugins::PluginErrorPlugin -- not found"
//         );

//         add_plugin(PluginErrorPlugin(self.flag.clone()), 10);

//         // 20 valid packet
//         // 1 lost to plugin panic
//         // 1 got through to trigger error
//         for i in 1..23 {
//             schedule_in(Message::new().build(), Duration::from_secs(i))
//         }
//     }

//     fn handle_message(&mut self, _msg: Message) {
//         if !self.flag.load(SeqCst) {
//             let err = PluginError::expected::<PluginErrorPlugin>();
//             assert_eq!(
//                 format!("{err}"),
//                 "expected plugin of type plugins::PluginErrorPlugin -- paniced"
//             );
//             self.done = true;
//         }
//     }

//     fn at_sim_end(&mut self) {
//         assert!(self.done)
//     }
// }

// #[test]
// #[serial]
// fn plugin_error_expected_t() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut rt = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut rt);

//     let module =
//         PluginErrorModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(
//         rt,
//         RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
//     );

//     let res = dbg!(rt.run());
//     let _res = res.unwrap();
//     // assert_eq!(res.3, 2);
// }

// struct PluginErrorTriggerPlugin(Arc<AtomicBool>);
// impl Plugin for PluginErrorTriggerPlugin {
//     fn capture(&mut self, msg: Option<Message>) -> Option<Message> {
//         if !self.0.load(SeqCst) {
//             let err = PluginError::expected::<PluginErrorPlugin>();
//             assert_eq!(
//                 format!("{err}"),
//                 "expected plugin of type plugins::PluginErrorPlugin -- paniced"
//             );
//         }
//         msg
//     }

//     fn defer(&mut self) {}
// }

// #[NdlModule]
// struct PluginErrorTriggerModule {
//     flag: Arc<AtomicBool>,
//     handles: Vec<PluginHandle>,
// }
// impl Module for PluginErrorTriggerModule {
//     fn new() -> Self {
//         Self {
//             flag: Arc::new(AtomicBool::new(false)),
//             handles: Vec::new(),
//         }
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         self.handles
//             .push(add_plugin(PluginErrorPlugin(self.flag.clone()), 10));
//         self.handles
//             .push(add_plugin(PluginErrorTriggerPlugin(self.flag.clone()), 100));

//         // 20 valid packet
//         // 1 lost to plugin panic
//         // 1 got through to trigger error
//         for i in 1..23 {
//             schedule_in(Message::new().build(), Duration::from_secs(i))
//         }
//     }

//     fn at_sim_end(&mut self) {
//         assert_eq!(plugin_status(&self.handles[0]), PluginStatus::Paniced);
//         assert_eq!(plugin_status(&self.handles[1]), PluginStatus::Active);
//     }
// }

// #[test]
// #[serial]
// fn plugin_error_expected_t_inside_other_plugin() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut rt = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut rt);

//     let module =
//         PluginErrorTriggerModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(
//         rt,
//         RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
//     );

//     let res = rt.run();
//     let _res = res.unwrap();
//     // assert_eq!(res.3, 2);
// }
