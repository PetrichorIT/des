use des::prelude::*;
use log::*;
use serial_test::serial;

#[test]
#[serial]
fn initalize_logger() {
    ScopedLogger::quiet()
        .finish()
        .expect("Failed to set logger");
}

#[test]
#[serial]
fn raw_logger() {
    ScopedLogger::quiet()
        .finish()
        .expect("Failed to set logger");

    info!("Hello World?");
    info!(target: "Module", "Hello World!");

    let scopes = ScopedLogger::yield_scopes();
    assert_eq!(scopes.len(), 1);

    let scope = scopes.get("Module").expect("Scoped missnamed");
    assert_eq!(*scope.target, "Module");
    assert_eq!(scope.stream.len(), 1);

    let record = scope.stream.front().expect("HUH");
    assert_eq!(record.time, SimTime::MIN);
    assert_eq!(*record.target, "Module");
    assert_eq!(record.level, Level::Info);
    assert_eq!(record.msg, "Hello World!");
}

#[NdlModule]
struct SomeModule {}

impl Module for SomeModule {
    fn at_sim_start(&mut self, stage: usize) {
        info!("at_sim_start_{}", stage);
        if stage == 1 {
            self.enable_activity(Duration::from_secs(2));
        }
    }

    fn num_sim_start_stages(&self) -> usize {
        2
    }

    fn activity(&mut self) {
        info!("activity");
        self.disable_activity();
        self.schedule_in(Message::new().build(), Duration::from_secs(2));
    }

    fn handle_message(&mut self, _msg: Message) {
        info!("handle_message");
    }

    fn at_sim_end(&mut self) {
        info!("at_sim_end");
    }
}

#[test]
#[serial]
fn module_auto_scopes() {
    ScopedLogger::quiet()
        .interal_max_log_level(LevelFilter::Warn)
        .finish()
        .expect("Failed to set logger");

    let mut rt = NetworkRuntime::new(());
    let globals = Ptr::downgrade(&rt.globals());

    let module_a = {
        let core = ModuleCore::new_with(
            ObjectPath::root_module("Module A".to_string()),
            globals.clone(),
        );

        Ptr::new(SomeModule::named(core))
    };
    rt.create_module(module_a);

    let module_b = {
        let core = ModuleCore::new_with(
            ObjectPath::root_module("Module B".to_string()),
            globals.clone(),
        );

        Ptr::new(SomeModule::named(core))
    };
    rt.create_module(module_b);

    let module_c = {
        let core = ModuleCore::new_with(
            ObjectPath::root_module("Module C".to_string()),
            globals.clone(),
        );

        Ptr::new(SomeModule::named(core))
    };
    rt.create_module(module_c);

    let runtime = Runtime::new(rt);
    match runtime.run() {
        RuntimeResult::Finished {
            time, event_count, ..
        } => {
            // Event Count
            // 1 SimStart
            // 3 x 1Activity
            // 3 x 1HandleMessage
            // == 7
            assert_eq!(event_count, 7);

            // Time
            // Delay 2s until activity + 2 until handle_message
            assert_eq_time!(time, 4.0);

            let scopes = ScopedLogger::yield_scopes();
            assert_eq!(scopes.len(), 3);
            for (trg, scope) in scopes {
                assert_eq!(trg, *scope.target);
                assert_eq!(scope.stream.len(), 5);

                assert!(["Module A", "Module B", "Module C"].contains(&&trg[..]));

                let mut last = SimTime::MIN;
                for msg in scope.stream {
                    assert_eq!(msg.target, scope.target);
                    assert!(last <= msg.time);
                    last = msg.time;
                }
            }
            // println!("{:?}", scopes);
            // panic!()
        }
        _ => panic!("Unexpected runtime result"),
    }
}
