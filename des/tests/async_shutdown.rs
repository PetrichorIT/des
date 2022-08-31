#![cfg(feature = "async")]
#![cfg(not(feature = "async-sharedrt"))]
use std::sync::atomic::{AtomicBool, Ordering};

use des::prelude::*;
use serial_test::serial;

struct DropTest {
    heap: Vec<usize>,
}

impl DropTest {
    fn new() -> Self {
        Self { heap: vec![0] }
    }

    fn step(&mut self) -> usize {
        let v = self.heap[self.heap.len() - 1];
        self.heap.push(v + 1);
        v + 1
    }
}

impl Drop for DropTest {
    fn drop(&mut self) {
        DROPPED.store(true, Ordering::SeqCst)
    }
}

#[NdlModule]
struct StatelessModule {}

#[async_trait::async_trait]
impl AsyncModule for StatelessModule {
    async fn at_sim_start(&mut self, _: usize) {
        tokio::spawn(async {
            let mut drop_test = DropTest::new();
            loop {
                tokio::sim::time::sleep(Duration::from_secs(1)).await;
                drop_test.step();
            }
        });
    }

    async fn handle_message(&mut self, _msg: Message) {
        self.shutdown(None);
    }
}

static DROPPED: AtomicBool = AtomicBool::new(false);

#[serial]
#[test]
fn stateless_module_shudown() {
    let mut rt = NetworkRuntime::new(());
    let mut module = StatelessModule::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let gate = module.create_gate("in", GateServiceType::Input, &mut rt);

    rt.create_module(module);
    let mut rt = Runtime::new(rt);
    rt.add_message_onto(
        gate,
        Message::new().build(),
        SimTime::from_duration(Duration::from_secs(10)),
    );

    let _ = rt.run();
    assert!(DROPPED.load(Ordering::SeqCst))
}
