#![cfg(feature = "net")]

use std::sync::Arc;
use std::sync::Mutex;

use des::net::BuildContext;
use des::net::__Buildable0;
use des::prelude::*;
use des::runtime::*;
use log::LevelFilter;
use serial_test::serial;

#[serial]
#[test]
fn initialize_logger() {
    Logger::new()
        .try_set_logger()
        .expect("Failed to create and attach logger")
}

#[derive(Debug, Clone, Default)]
struct DebugOutput {
    inner: Arc<Mutex<Vec<String>>>,
}

impl LogOutput for DebugOutput {
    fn write(&mut self, record: &LogRecord, fmt: LogFormat) -> std::io::Result<()> {
        self.inner.lock().unwrap().write(record, fmt)
    }
}

#[NdlModule]
struct Counter {
    i: i32,
}

impl Module for Counter {
    fn new() -> Self {
        Self { i: 0 }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        schedule_in(Message::new().build(), Duration::from_secs(0));
    }

    fn handle_message(&mut self, _msg: Message) {
        match self.i % 5 {
            0 => log::trace!("{}", self.i),
            1 => log::debug!("{}", self.i),
            2 => log::info!("{}", self.i),
            3 => log::warn!("{}", self.i),
            4 => log::error!("{}", self.i),
            _ => unreachable!(),
        };
        self.i += 1;
        schedule_in(Message::new().build(), Duration::from_secs(1));
    }
}

#[serial]
#[test]
fn one_module_linear_logger() {
    let output = DebugOutput::default();
    struct DebugPolicy {
        output: DebugOutput,
    }
    impl LogScopeConfigurationPolicy for DebugPolicy {
        fn configure(&self, _scope: &str) -> (Box<dyn LogOutput>, LogFormat) {
            (Box::new(self.output.clone()), LogFormat::NoColor)
        }
    }

    Logger::new()
        .interal_max_log_level(LevelFilter::Warn)
        .policy_object(DebugPolicy {
            output: output.clone(),
        })
        .try_set_logger()
        .unwrap();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let module = Counter::build_named(ObjectPath::root_module("modpath"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        app,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );
    let _ = rt.run().unwrap_premature_abort();

    let lock = output.inner.lock().unwrap();
    assert_eq!(lock.len(), 31);
    // println!("{lock:?}");
    for i in 0..31 {
        let level = match i % 5 {
            0 => "TRACE",
            1 => "DEBUG",
            2 => "INFO",
            3 => "WARN",
            4 => "ERROR",
            _ => unreachable!(),
        };
        assert_eq!(
            lock[i],
            format!(
                "[ {:^5} ] {} modpath: {}\n",
                SimTime::from_duration(Duration::from_secs(i as u64)),
                level,
                i
            )
        )
    }
}

#[serial]
#[test]
fn multiple_module_linear_logger() {
    let output0 = DebugOutput::default();
    let output1 = DebugOutput::default();
    struct DebugPolicy {
        output0: DebugOutput,
        output1: DebugOutput,
    }
    impl LogScopeConfigurationPolicy for DebugPolicy {
        fn configure(&self, scope: &str) -> (Box<dyn LogOutput>, LogFormat) {
            match scope {
                "node0" => (Box::new(self.output0.clone()), LogFormat::NoColor),
                "node1" => (Box::new(self.output1.clone()), LogFormat::NoColor),
                _ => unreachable!(),
            }
        }
    }

    Logger::new()
        .interal_max_log_level(LevelFilter::Warn)
        .policy_object(DebugPolicy {
            output0: output0.clone(),
            output1: output1.clone(),
        })
        .try_set_logger()
        .unwrap();

    let mut app = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut app);

    let node0 = Counter::build_named(ObjectPath::root_module("node0"), &mut cx);
    cx.create_module(node0);

    let node1 = Counter::build_named(ObjectPath::root_module("node1"), &mut cx);
    cx.create_module(node1);

    let rt = Runtime::new_with(
        app,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(30))),
    );
    let _ = rt.run().unwrap_premature_abort();

    for (output, path) in [(output0, "node0"), (output1, "node1")] {
        let lock = output.inner.lock().unwrap();
        assert_eq!(lock.len(), 31);
        // println!("{lock:?}");
        for i in 0..31 {
            let level = match i % 5 {
                0 => "TRACE",
                1 => "DEBUG",
                2 => "INFO",
                3 => "WARN",
                4 => "ERROR",
                _ => unreachable!(),
            };
            assert_eq!(
                lock[i],
                format!(
                    "[ {:^5} ] {} {}: {}\n",
                    SimTime::from_duration(Duration::from_secs(i as u64)),
                    level,
                    path,
                    i
                )
            )
        }
    }
}
