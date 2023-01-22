#![cfg(feature = "net")]
use des::net::{plugin::*, BuildContext, __Buildable0};
use des::prelude::*;
use serial_test::serial;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

mod common {
    use des::net::plugin::*;
    use des::prelude::*;

    pub struct ConsumeAllIncoming;
    impl Plugin for ConsumeAllIncoming {
        fn capture_incoming(&mut self, _msg: Message) -> Option<Message> {
            None
        }
    }

    pub struct ConsumeAllOutgoing;
    impl Plugin for ConsumeAllOutgoing {
        fn capture_outgoing(&mut self, _: Message) -> Option<Message> {
            None
        }

        fn event_end(&mut self) {
            log::info!("consumed_outgoing");
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
        assert_eq!(self.handles[0].status(), PluginStatus::StartingUp);
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
fn plugin_raw_creation() {
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
            let p = add_plugin(
                Self {
                    level: self.level + 1,
                },
                self.level as usize + 1,
            );
            assert_eq!(p.status(), PluginStatus::StartingUp);
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
        let id = msg.header().id; // number of modules that are active
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

// #[test]
// #[serial]
// #[should_panic = "common::PanicOnIncoming"]
// fn plugin_panic_abort() {
//     // Logger::new().set_logger();

//     let mut app = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut app);

//     let module = PanicPolicyAbort::build_named(ObjectPath::root_module("root"), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
//     let _result = rt.run();

//     panic!("Should never have reached this point")
// }

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

struct PluginErrorPlugin(Arc<AtomicBool>);
impl Plugin for PluginErrorPlugin {
    fn event_start(&mut self) {
        if SimTime::now().as_secs() > 20 {
            panic!("Test-Panic to get plugin error")
        }
        self.0.store(true, SeqCst);
    }

    fn event_end(&mut self) {
        self.0.store(false, SeqCst)
    }
}

#[NdlModule]
struct PluginErrorModule {
    flag: Arc<AtomicBool>,
    done: bool,
}
impl Module for PluginErrorModule {
    fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            done: false,
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        let err = PluginError::expected::<PluginErrorPlugin>();
        assert_eq!(
            format!("{err}"),
            "expected plugin of type plugins::PluginErrorPlugin, but no such plugin exists (ENOTFOUND)"
        );

        add_plugin(PluginErrorPlugin(self.flag.clone()), 10);

        // 20 valid packet
        // 1 lost to plugin panic
        // 1 got through to trigger error
        for i in 1..23 {
            schedule_in(Message::new().build(), Duration::from_secs(i))
        }
    }

    fn handle_message(&mut self, _msg: Message) {
        if !self.flag.load(SeqCst) {
            let err = PluginError::expected::<PluginErrorPlugin>();
            assert_eq!(
                format!("{err}"),
                "expected plugin of type plugins::PluginErrorPlugin was found, but paniced (EPANICED)"
            );
            self.done = true;
        }
    }

    fn at_sim_end(&mut self) {
        assert!(self.done)
    }
}

#[test]
#[serial]
fn plugin_error_expected_t() {
    // Logger::new().set_logger();

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module =
        PluginErrorModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );

    let res = rt.run();
    let _res = res.unwrap();
    // assert_eq!(res.3, 2);
}

struct PluginErrorTriggerPlugin(Arc<AtomicBool>);
impl Plugin for PluginErrorTriggerPlugin {
    fn event_start(&mut self) {
        if !self.0.load(SeqCst) {
            let err = PluginError::expected::<PluginErrorPlugin>();
            assert_eq!(
                format!("{err}"),
                "expected plugin of type plugins::PluginErrorPlugin was found, but paniced (EPANICED)"
            );
        }
    }
}

#[NdlModule]
struct PluginErrorTriggerModule {
    flag: Arc<AtomicBool>,
    handles: Vec<PluginHandle>,
}
impl Module for PluginErrorTriggerModule {
    fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            handles: Vec::new(),
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        self.handles
            .push(add_plugin(PluginErrorPlugin(self.flag.clone()), 10));
        self.handles
            .push(add_plugin(PluginErrorTriggerPlugin(self.flag.clone()), 100));

        // 20 valid packet
        // 1 lost to plugin panic
        // 1 got through to trigger error
        for i in 1..23 {
            schedule_in(Message::new().build(), Duration::from_secs(i))
        }
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.handles[0].status(), PluginStatus::Paniced);
        assert_eq!(self.handles[1].status(), PluginStatus::Active);
    }
}

#[test]
#[serial]
fn plugin_error_expected_t_inside_other_plugin() {
    // Logger::new().set_logger();

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module =
        PluginErrorTriggerModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );

    let res = rt.run();
    let _res = res.unwrap();
    // assert_eq!(res.3, 2);
}

struct ExpectedPlugin;
impl Plugin for ExpectedPlugin {
    fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
        // We expect ExpectingPlugin and want a priority error
        let err = PluginError::expected::<ExpectingPlugin>();
        assert_eq!(err.kind(), PluginErrorKind::PluginWithLowerPriority, "{err}");
        assert_eq!(
            format!("{err}"),
            "expected plugin of type plugins::ExpectingPlugin was found, but not yet active due to priority (EINACTIVE)"
        );
        Some(msg)
    }
}

struct ExpectingPlugin;
impl Plugin for ExpectingPlugin {
    fn capture_incoming(&mut self, msg: Message) -> Option<Message> {
        // We expect ExpectingPlugin and want a priority error
        let err = PluginError::expected::<ExpectedPlugin>();
        assert_eq!(err.kind(), PluginErrorKind::PluginMalfunction);
        assert_eq!(
            format!("{err}"),
            "expected plugin of type plugins::ExpectedPlugin was found, but malfunctioned (EMALFUNCTION)"
        );

        let err = PluginError::expected::<Self>();
        assert_eq!(err.kind(), PluginErrorKind::PluginMalfunction);
        assert_eq!(
            format!("{err}"),
            "expected plugin of type plugins::ExpectingPlugin was found, but is self (EMALFUNCTION)"
        );
        Some(msg)
    }
}

#[NdlModule]
struct PluginErrorMalfunction {
    done: bool,
}
impl Module for PluginErrorMalfunction {
    fn new() -> Self {
        Self { done: false }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin_with(ExpectedPlugin, 10, PluginPanicPolicy::Abort);
        add_plugin_with(ExpectingPlugin, 100, PluginPanicPolicy::Abort);

        schedule_in(Message::new().build(), Duration::from_secs(1));
    }

    fn handle_message(&mut self, _msg: Message) {
        self.done = true;
    }

    fn at_sim_end(&mut self) {
        assert!(self.done);
    }
}

#[test]
#[serial]
fn plugin_error_malfunction_or_priority() {
    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module =
        PluginErrorMalfunction::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );

    let res = rt.run();
    let _res = res.unwrap();
}

#[NdlModule]
struct PluginOutputCapture {
    c: usize,
}
impl Module for PluginOutputCapture {
    fn new() -> Self {
        Self { c: 0 }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        add_plugin(common::ConsumeAllOutgoing, 100);
        // This packet will go through
        schedule_in(Message::new().id(0).build(), Duration::from_secs(0 as u64));
    }

    fn handle_message(&mut self, msg: Message) {
        self.c += 1;
        if msg.header().id == 0 {
            for i in 1..100 {
                schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64));
            }
        }
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.c, 1);
    }
}

#[NdlModule]
struct PluginRemovalFromMain {
    handles: Vec<PluginHandle>,
}
impl Module for PluginRemovalFromMain {
    fn new() -> Self {
        Self {
            handles: Vec::new(),
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        for i in 1..=10 {
            self.handles
                .push(add_plugin(common::IncrementIncomingId, i))
        }

        for i in 0..=10 {
            schedule_in(
                Message::new().kind(i).build(),
                Duration::from_secs(i as u64),
            )
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let t = SimTime::now().as_secs();
        let id = 10 - t;
        assert_eq!(msg.header().id as u64, id);
        assert_eq!(self.handles.len() as u64, id);

        if let Some(h) = self.handles.pop() {
            h.remove();
        }
    }

    fn at_sim_end(&mut self) {
        assert!(self.handles.is_empty())
    }
}

#[test]
#[serial]
fn plugin_removal_from_main() {
    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module =
        PluginRemovalFromMain::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );

    let res = rt.run();
    let _res = res.unwrap();
}

struct RemoveChildAtLevel {
    child: Option<PluginHandle>,
    level: u16,
}
impl Plugin for RemoveChildAtLevel {
    fn capture_incoming(&mut self, mut msg: Message) -> Option<Message> {
        if msg.header().id == self.level {
            log::debug!("killing child");
            self.child.take().map(PluginHandle::remove);
        }
        msg.header_mut().kind += 1;
        Some(msg)
    }

    fn capture_outgoing(&mut self, _msg: Message) -> Option<Message> {
        unreachable!()
    }
}

#[NdlModule]
struct PluginRemovalFromUpstream {
    handle: Option<PluginHandle>,
}
impl Module for PluginRemovalFromUpstream {
    fn new() -> Self {
        Self { handle: None }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        let mut last = None;
        for i in 1..=10 {
            last = Some(add_plugin(
                RemoveChildAtLevel {
                    child: last.take(),
                    level: i,
                },
                i as usize,
            ));

            schedule_in(
                Message::new().id(i).kind(0).build(),
                Duration::from_secs(i as u64),
            );
        }
        self.handle = last;
    }

    fn handle_message(&mut self, mut msg: Message) {
        log::info!("received: {:?}", msg);
        let t = SimTime::now().as_secs();
        if t == 1 {
            // emualte del of nonexitsting element in the chain
            msg.header_mut().kind += 1;
        }
        let id = 12 - t;
        assert_eq!(msg.header().kind as u64, id);
        assert_eq!(self.handle.as_ref().unwrap().status(), PluginStatus::Active);
    }
}

#[test]
#[serial]
fn plugin_removal_from_upstream() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = PluginRemovalFromUpstream::build_named(
        ObjectPath::root_module("root".to_string()),
        &mut cx,
    );
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );

    let res = rt.run();
    let _res = res.unwrap();
}

struct RemoveChildAtLevelDownstream {
    child: Option<PluginHandle>,
    level: u16,
}
impl Plugin for RemoveChildAtLevelDownstream {
    fn capture_outgoing(&mut self, mut msg: Message) -> Option<Message> {
        if msg.header().id == self.level {
            log::debug!("killing child");
            self.child.take().map(PluginHandle::remove);
        }
        msg.header_mut().kind += 1;
        Some(msg)
    }
}

#[NdlModule]
struct PluginRemovalFromDownstream {
    handle: Option<PluginHandle>,
    done: bool,
}
impl Module for PluginRemovalFromDownstream {
    fn new() -> Self {
        Self {
            handle: None,
            done: false,
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        let mut last = Some(add_plugin(
            RemoveChildAtLevelDownstream {
                child: None,
                level: 200,
            },
            1000,
        ));
        for i in 1..=10 {
            last = Some(add_plugin(
                RemoveChildAtLevelDownstream {
                    child: last.take(),
                    level: i,
                },
                20 - i as usize,
            ));
        }
        schedule_in(
            Message::new().kind(42).content("").build(),
            Duration::from_secs(1),
        );
        self.handle = last;
    }

    fn handle_message(&mut self, msg: Message) {
        match msg.header().kind {
            42 => {
                // we want to trigger a close of downstream parsers.
                log::info!("starting");
                schedule_in(
                    Message::new().id(1).content(true).build(),
                    Duration::from_secs(1),
                );
                return;
            }
            1 => {
                self.done = true;
            }
            n => {
                let t = SimTime::now().as_secs();
                let kind = 13 - t;
                assert_eq!(n as u64, kind);

                log::info!("sending");
                schedule_in(
                    Message::new().id(msg.header().id + 1).content(true).build(),
                    Duration::from_secs(1),
                );
            }
        }
    }

    fn at_sim_end(&mut self) {
        assert!(self.done)
    }
}

#[test]
#[serial]
fn plugin_removal_from_downstream() {
    // Logger::new()
    //     .interal_max_log_level(log::LevelFilter::Trace)
    //     .set_logger();

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = PluginRemovalFromDownstream::build_named(
        ObjectPath::root_module("root".to_string()),
        &mut cx,
    );
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );

    let res = rt.run();
    let _res = res.unwrap();
}

// #[test]
// #[serial]
// fn plugin_output_capture_consume_from_main() {
//     let mut rt = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut rt);

//     let module =
//         PluginOutputCapture::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(
//         rt,
//         RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
//     );

//     let res = rt.run();
//     let res = res.unwrap();

//     assert_eq!(res.1.as_secs(), 0);
// }

// struct EmitAtEventEnd;
// impl Plugin for EmitAtEventEnd {
//     fn event_end(&mut self) {
//         log::info!("emit:at:end");
//         schedule_in(Message::new().build(), Duration::from_secs(1));
//     }
// }

// #[NdlModule]
// struct PluginOutputCaptureScoping {
//     c: usize,
// }
// impl Module for PluginOutputCaptureScoping {
//     fn new() -> Self {
//         Self { c: 0 }
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         add_plugin(common::ConsumeAllOutgoing, 100);
//         add_plugin(EmitAtEventEnd, 10);
//         // This packet will go through
//         schedule_in(
//             Message::new().id(255).build(),
//             Duration::from_secs(1 as u64),
//         );
//     }

//     fn handle_message(&mut self, msg: Message) {
//         log::debug!("{:?}", msg.header());
//         self.c += 1;

//         for i in 1..10 {
//             schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64));
//         }
//     }

//     fn at_sim_end(&mut self) {
//         assert_eq!(self.c, 30);
//     }
// }

// #[test]
// #[serial]
// fn plugin_output_capture_consume_from_plugin() {
//     // Logger::new().set_logger();

//     let mut rt = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut rt);

//     let module = PluginOutputCaptureScoping::build_named(
//         ObjectPath::root_module("root".to_string()),
//         &mut cx,
//     );
//     cx.create_module(module);

//     let rt = Runtime::new_with(
//         rt,
//         RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))), // .max_itr(1),
//     );

//     let res = rt.run();
//     let res = res.unwrap_premature_abort();

//     assert_eq!(res.1.as_secs(), 30);
// }
