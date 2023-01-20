// #![cfg(feature = "net")]
// use des::{
//     net::{
//         BuildContext, __Buildable0,
//         plugin::{PeriodicPlugin, PluginError, PluginHandle, PluginStatus},
//     },
//     prelude::*,
// };
// use serial_test::serial;
// use std::sync::{
//     atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
//     Arc,
// };

// mod sample {
//     use std::sync::atomic::Ordering::SeqCst;
//     use std::sync::{atomic::AtomicUsize, Arc};

//     use des::prelude::*;

//     pub struct ConsumeAll;
//     impl Plugin for ConsumeAll {
//         fn capture(&mut self, _: Option<Message>) -> Option<Message> {
//             None
//         }
//         fn defer(&mut self) {}
//     }

//     pub struct PanicOnMessage;
//     impl Plugin for PanicOnMessage {
//         fn capture(&mut self, msg: Option<Message>) -> Option<Message> {
//             if msg.is_some() {
//                 panic!("PanicOnMessage received message")
//             } else {
//                 None
//             }
//         }
//         fn defer(&mut self) {}
//     }

//     pub struct ActivitySensor {
//         pub expected: usize,
//         pub arc: Arc<AtomicUsize>,
//     }

//     impl Plugin for ActivitySensor {
//         fn capture(&mut self, msg: Option<Message>) -> Option<Message> {
//             // log::debug!("INC #{}", self.expected);
//             let real = self.arc.fetch_add(1, SeqCst);
//             assert_eq!(real, self.expected);
//             msg
//         }

//         fn defer(&mut self) {
//             // log::debug!("DEC #{}", self.expected);
//             let real = self.arc.fetch_sub(1, SeqCst);
//             assert_eq!(real - 1, self.expected);
//         }
//     }

//     pub struct IncrementId;
//     impl Plugin for IncrementId {
//         fn capture(&mut self, mut msg: Option<Message>) -> Option<Message> {
//             msg.as_mut()?.header_mut().id += 1;
//             msg
//         }

//         fn defer(&mut self) {}
//     }
// }

// #[NdlModule]
// struct PluginPriority;
// impl Module for PluginPriority {
//     fn new() -> Self {
//         Self
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         add_plugin(sample::PanicOnMessage, 100);
//         add_plugin(sample::ConsumeAll, 10);

//         for i in 0..100 {
//             schedule_in(Message::new().build(), Duration::from_secs(i));
//         }
//     }

//     fn handle_message(&mut self, _msg: Message) {
//         panic!("Panic on message plugin let through message")
//     }
// }

// #[test]
// #[serial]
// fn plugin_priority() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut app = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut app);

//     let module = PluginPriority::build_named(ObjectPath::root_module("root"), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
//     let result = rt.run();

//     let RuntimeResult::Finished { time, profiler, .. } = result else {
//         panic!("Unexpected runtime result")
//     };

//     assert_eq!(time, 99.0);
//     assert_eq!(profiler.event_count, 100 + 1);
// }

// #[NdlModule]
// struct PluginPriorityDefer {
//     arc: Arc<AtomicUsize>,
// }
// impl Module for PluginPriorityDefer {
//     fn new() -> Self {
//         Self {
//             arc: Arc::new(AtomicUsize::new(0)),
//         }
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         add_plugin(
//             sample::ActivitySensor {
//                 arc: self.arc.clone(),
//                 expected: 1,
//             },
//             100,
//         );
//         add_plugin(
//             sample::ActivitySensor {
//                 arc: self.arc.clone(),
//                 expected: 0,
//             },
//             10,
//         );
//         add_plugin(
//             sample::ActivitySensor {
//                 arc: self.arc.clone(),
//                 expected: 2,
//             },
//             1000,
//         );

//         for i in 0..100 {
//             schedule_in(Message::new().build(), Duration::from_secs(i));
//         }
//     }

//     fn handle_message(&mut self, _msg: Message) {}
// }

// #[test]
// #[serial]
// fn plugin_priority_defer() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut app = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut app);

//     let module = PluginPriorityDefer::build_named(ObjectPath::root_module("root"), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
//     let result = rt.run();

//     let RuntimeResult::Finished { time, profiler, .. } = result else {
//         panic!("Unexpected runtime result")
//     };

//     assert_eq!(time, 99.0);
//     assert_eq!(profiler.event_count, 100 + 1);
// }

// #[NdlModule]
// struct PluginDuplication {
//     counter: usize,
// }
// impl Module for PluginDuplication {
//     fn new() -> Self {
//         Self { counter: 0 }
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         add_plugin(sample::IncrementId, 100);
//         add_plugin(sample::IncrementId, 1000);

//         for i in 0..100 {
//             schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64));
//         }
//     }

//     fn handle_message(&mut self, msg: Message) {
//         let id = msg.header().id as usize;
//         assert_eq!(id, self.counter + 2);
//         self.counter += 1;
//     }

//     fn at_sim_end(&mut self) {
//         assert_eq!(self.counter, 100)
//     }
// }

// #[test]
// #[serial]
// fn plugin_duplication() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut app = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut app);

//     let module = PluginDuplication::build_named(ObjectPath::root_module("root"), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
//     let result = rt.run();

//     let RuntimeResult::Finished { time, profiler, .. } = result else {
//         panic!("Unexpected runtime result")
//     };

//     assert_eq!(time, 99.0);
//     assert_eq!(profiler.event_count, 100 + 1);
// }

// #[NdlModule]
// struct PluginRemoval {
//     counter: usize,
//     handle: Option<PluginHandle>,
// }
// impl Module for PluginRemoval {
//     fn new() -> Self {
//         Self {
//             counter: 0,
//             handle: None,
//         }
//     }

//     fn at_sim_start(&mut self, _stage: usize) {
//         self.handle = Some(add_plugin(sample::IncrementId, 100));
//         add_plugin(sample::IncrementId, 1000);

//         for i in 0..100 {
//             schedule_in(Message::new().id(i).build(), Duration::from_secs(i as u64));
//         }

//         schedule_in(Message::new().kind(42).build(), Duration::from_secs(123));

//         for i in 0..100 {
//             schedule_in(
//                 Message::new().id(200 + i).build(),
//                 Duration::from_secs(200 + i as u64),
//             );
//         }
//     }

//     fn handle_message(&mut self, msg: Message) {
//         if msg.header().kind == 42 {
//             assert_eq!(self.counter, 100);
//             remove_plugin(self.handle.take().unwrap());
//             self.counter = 199;
//             return;
//         }

//         let id = msg.header().id as usize;
//         assert_eq!(id, self.counter + 2);
//         self.counter += 1;
//     }

//     fn at_sim_end(&mut self) {
//         assert_eq!(self.counter, 299);
//     }
// }

// #[test]
// #[serial]
// fn plugin_removal() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut app = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut app);

//     let module = PluginRemoval::build_named(ObjectPath::root_module("root"), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
//     let result = rt.run();

//     let RuntimeResult::Finished { time, profiler, .. } = result else {
//         panic!("Unexpected runtime result")
//     };

//     assert_eq!(time, 299.0);
//     assert_eq!(profiler.event_count, 201 + 1);
// }

// #[NdlModule]
// struct PluginInPluginAdd;
// impl Module for PluginInPluginAdd {
//     fn new() -> Self {
//         Self
//     }
//     fn at_sim_start(&mut self, _stage: usize) {
//         add_plugin(RecursivePlugin, 69);
//     }
// }

// struct RecursivePlugin;
// impl Plugin for RecursivePlugin {
//     fn capture_sim_start(&mut self) {
//         add_plugin(Self, 42);
//     }
//     fn capture(&mut self, msg: Option<Message>) -> Option<Message> {
//         msg
//     }
//     fn defer(&mut self) {}
// }

// #[test]
// #[serial]
// fn plugin_in_plugin_add() {
//     // ScopedLogger::new().finish().unwrap();

//     let mut app = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut app);

//     let module = PluginInPluginAdd::build_named(ObjectPath::root_module("root"), &mut cx);
//     cx.create_module(module);

//     let rt = Runtime::new_with(app, RuntimeOptions::seeded(123));
//     let _result = rt.run();

//     // should rach this point;
//     // panic!("Should not have reached this point");
// }

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
