use std::sync::{atomic::AtomicUsize, Arc};

use des::{
    net::{hooks::PeriodicHook, BuildContext, __Buildable0},
    prelude::*,
};

#[NdlModule]
struct PeriodicModule {
    state: Arc<AtomicUsize>,
}

impl Module for PeriodicModule {
    fn new() -> Self {
        PeriodicModule {
            state: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        create_hook(PeriodicHook::new(
            |state| {
                state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            },
            Duration::from_secs(1),
            self.state.clone(),
        ))
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!("This function should never be called")
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.state.load(std::sync::atomic::Ordering::SeqCst), 10)
    }
}

#[test]
fn periodic_hook() {
    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = PeriodicModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(10))),
    );

    let res = dbg!(rt.run());
    let res = res.unwrap_premature_abort();
    assert_eq!(res.3, 1)
}
