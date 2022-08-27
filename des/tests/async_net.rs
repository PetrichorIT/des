#![cfg(feature = "async")]
use des::prelude::*;
use des::tokio::sim::net::*;
use serial_test::serial;
use std::net::Ipv4Addr;

#[NdlModule]
struct TcpBindTests {}

#[async_trait::async_trait]
impl AsyncModule for TcpBindTests {
    async fn at_sim_start(&mut self, _: usize) {
        // Set IO Context
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 112));

        let s0 = TcpListener::bind("192.168.2.112:8000").await.unwrap();
    }

    async fn handle_message(&mut self, _: Message) {}
}

#[test]
#[serial]
fn tcp_listener_binds() {
    let mut rt = NetworkRuntime::new(());
    let module = TcpBindTests::named_root(ModuleCore::new_with(
        ObjectPath::root_module("RootModule".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    rt.create_module(module);

    let rt = Runtime::new(rt);
    let _ = rt.run();
}
