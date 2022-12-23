use std::sync::{atomic::AtomicUsize, Arc};

use des::{
    net::{
        hooks::{Hook, HookHandle, PeriodicHook, RoutingHook, RoutingHookOptions},
        BuildContext, __Buildable0,
    },
    prelude::*,
};
use serial_test::serial;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{IOContext, TcpListener, TcpStream},
    task::JoinHandle,
};

struct ConsumAllHook;
impl Hook for ConsumAllHook {
    fn handle_message(&mut self, _: Message) -> Result<(), Message> {
        log::info!("Consumed message");
        Ok(())
    }
}

struct PanicAllHook;
impl Hook for PanicAllHook {
    fn handle_message(&mut self, _: Message) -> Result<(), Message> {
        panic!("This hook should never get any message")
    }
}

#[NdlModule]
struct HookPriorityModule {}

impl Module for HookPriorityModule {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        create_hook(
            PeriodicHook::new(
                |_| schedule_in(Message::new().build(), Duration::from_secs(1)),
                Duration::from_secs(1),
                (),
            ),
            0,
        );

        create_hook(PanicAllHook, 100);
        create_hook(ConsumAllHook, 10);
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!("All messages should have been processed by the hooks")
    }
}

#[test]
#[serial]
fn hook_priority() {
    // ScopedLogger::new().finish().unwrap();

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = HookPriorityModule::build_named(ObjectPath::root_module("root"), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from(10.0)),
    );
    let _ = rt.run();
}

#[NdlModule]
struct HookAtShutdown {
    state: Arc<AtomicUsize>,
}

impl Module for HookAtShutdown {
    fn new() -> Self {
        Self {
            state: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        create_hook(
            PeriodicHook::new(
                |state| {
                    state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if SimTime::now().as_secs() == 5 {
                        shutdow_and_restart_in(Duration::from_secs(2));
                    }
                },
                Duration::from_secs(1),
                self.state.clone(),
            ),
            0,
        );
    }

    fn handle_message(&mut self, msg: Message) {
        dbg!(msg);
        panic!("This function should never be called")
    }

    fn at_sim_end(&mut self) {
        // at 1,2,3,4,5 .. 8,9,10
        assert_eq!(self.state.load(std::sync::atomic::Ordering::SeqCst), 5 + 3)
    }
}

#[test]
fn hook_at_shutdown() {
    // ScopedLogger::new().finish().unwrap();

    // run 5s shutdown for 3s run 2s
    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module = HookAtShutdown::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(10))),
    );

    let res = dbg!(rt.run());
    let res = res.unwrap_premature_abort();
    assert_eq!(res.3, 1)
}

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
        create_hook(
            PeriodicHook::new(
                |state| {
                    state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                },
                Duration::from_secs(1),
                self.state.clone(),
            ),
            0,
        );
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!("This function should never be called")
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.state.load(std::sync::atomic::Ordering::SeqCst), 10)
    }
}

#[test]
#[serial]
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

#[NdlModule]
struct PeriodicMultiModule {
    state: Arc<AtomicUsize>,
}

impl Module for PeriodicMultiModule {
    fn new() -> Self {
        PeriodicMultiModule {
            state: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        create_hook(
            PeriodicHook::new(
                |state| {
                    state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                },
                Duration::from_secs(1),
                self.state.clone(),
            ),
            0,
        );
        create_hook(
            PeriodicHook::new(
                |state| {
                    state.fetch_add(2, std::sync::atomic::Ordering::SeqCst);
                },
                Duration::from_secs(2),
                self.state.clone(),
            ),
            0,
        );
    }

    fn handle_message(&mut self, _msg: Message) {
        panic!("This function should never be called")
    }

    fn at_sim_end(&mut self) {
        assert_eq!(self.state.load(std::sync::atomic::Ordering::SeqCst), 20)
    }
}

#[test]
#[serial]
fn periodic_hook_multiple() {
    // ScopedLogger::new().finish().unwrap();

    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);

    let module =
        PeriodicMultiModule::build_named(ObjectPath::root_module("root".to_string()), &mut cx);
    cx.create_module(module);

    let rt = Runtime::new_with(
        rt,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(10))),
    );

    let res = dbg!(rt.run());
    let res = res.unwrap_premature_abort();
    assert_eq!(res.3, 2);
}

#[NdlModule]
struct Router {}

impl Module for Router {
    fn new() -> Self {
        Self {}
    }
    fn at_sim_start(&mut self, stage: usize) {
        if stage == 1 {
            let ip = Ipv4Addr::from(random::<u32>());
            assert!(!ip.is_loopback() && !ip.is_unspecified());
            IOContext::new(random::<[u8; 6]>(), ip).set();

            log::info!("Router with addr {}", ip);

            create_hook(RoutingHook::new(RoutingHookOptions::INET), 0);
        }
    }

    fn num_sim_start_stages(&self) -> usize {
        2
    }
}

#[NdlModule]
struct TcpClient {
    handle: Option<JoinHandle<()>>,
}

#[async_trait::async_trait]
impl AsyncModule for TcpClient {
    fn new() -> Self {
        Self { handle: None }
    }

    async fn at_sim_start(&mut self, _stage: usize) {
        let ip = Ipv4Addr::from(random::<u32>());
        assert!(!ip.is_loopback() && !ip.is_unspecified());
        IOContext::new(random::<[u8; 6]>(), ip).set();

        log::info!("Client with addr {}", ip);

        self.handle = Some(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs_f64(10.0 * random::<f64>())).await;
            let mut stream = TcpStream::connect("1.1.1.1:8000").await.unwrap();
            stream.write_all(b"Hello World").await.unwrap();
        }));
    }

    async fn at_sim_end(&mut self) {
        self.handle.take().unwrap().await.unwrap();
    }
}

#[NdlModule]
struct TcpServer {}

#[async_trait::async_trait]
impl AsyncModule for TcpServer {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _stage: usize) {
        IOContext::new(random::<[u8; 6]>(), Ipv4Addr::new(1, 1, 1, 1)).set();

        tokio::spawn(async {
            let lis = TcpListener::bind("0.0.0.0:8000").await.unwrap();
            while let Ok((mut con, _from)) = lis.accept().await {
                let mut buf = [0u8; 512];
                let n = con.read(&mut buf).await.unwrap();

                let _ = n;
            }
        });
    }
}

// #[test]
// #[serial]
// fn routing_hook() {
//     ScopedLogger::new().finish().unwrap();

//     let mut rt = NetworkRuntime::new(());
//     let mut cx = BuildContext::new(&mut rt);

//     let client_a = TcpClient::build_named(ObjectPath::root_module("client_a"), &mut cx);
//     let client_b = TcpClient::build_named(ObjectPath::root_module("client_b"), &mut cx);
//     let client_c = TcpClient::build_named(ObjectPath::root_module("client_c"), &mut cx);

//     let router = Router::build_named(ObjectPath::root_module("router"), &mut cx);

//     let server = TcpServer::build_named(ObjectPath::root_module("server"), &mut cx);

//     // client to router

//     let a_out = client_a.create_gate("out", GateServiceType::Output);
//     let b_out = client_b.create_gate("out", GateServiceType::Output);
//     let c_out = client_c.create_gate("out", GateServiceType::Output);

//     let port_a_in = router.create_gate("port_a_in", GateServiceType::Input);
//     let port_b_in = router.create_gate("port_b_in", GateServiceType::Input);
//     let port_c_in = router.create_gate("port_c_in", GateServiceType::Input);

//     let chan_a = Channel::new(
//         ObjectPath::channel_with("chan", &client_a.path()),
//         ChannelMetrics::new_with_cost(
//             1000000,
//             Duration::from_millis(10),
//             Duration::ZERO,
//             1.0,
//             4096,
//         ),
//     );
//     let chan_b = Channel::new(
//         ObjectPath::channel_with("chan", &client_b.path()),
//         ChannelMetrics::new_with_cost(
//             1000000,
//             Duration::from_millis(10),
//             Duration::ZERO,
//             1.0,
//             4096,
//         ),
//     );
//     let chan_c = Channel::new(
//         ObjectPath::channel_with("chan", &client_c.path()),
//         ChannelMetrics::new_with_cost(
//             1000000,
//             Duration::from_millis(10),
//             Duration::ZERO,
//             1.0,
//             4096,
//         ),
//     );

//     a_out.set_next_gate(port_a_in);
//     a_out.set_channel(chan_a.clone());

//     b_out.set_next_gate(port_b_in);
//     b_out.set_channel(chan_b.clone());

//     c_out.set_next_gate(port_c_in);
//     c_out.set_channel(chan_c.clone());

//     // router to client

//     let port_a_out = router.create_gate("port_a_out", GateServiceType::Output);
//     let port_b_out = router.create_gate("port_b_out", GateServiceType::Output);
//     let port_c_out = router.create_gate("port_c_out", GateServiceType::Output);

//     let a_in = client_a.create_gate("in", GateServiceType::Input);
//     let b_in = client_b.create_gate("in", GateServiceType::Input);
//     let c_in = client_c.create_gate("in", GateServiceType::Input);

//     port_a_out.set_next_gate(a_in);
//     port_b_out.set_next_gate(b_in);
//     port_c_out.set_next_gate(c_in);

//     // server connections

//     let server_in = server.create_gate("in", GateServiceType::Input);
//     let server_out = server.create_gate("out", GateServiceType::Output);

//     let port_s_in = router.create_gate("port_s_in", GateServiceType::Input);
//     let port_s_out = router.create_gate("port_s_out", GateServiceType::Output);

//     let chan_s = Channel::new(
//         ObjectPath::channel_with("chan", &server.path()),
//         ChannelMetrics::new_with_cost(
//             1000000,
//             Duration::from_millis(10),
//             Duration::ZERO,
//             1.0,
//             4096,
//         ),
//     );

//     server_out.set_next_gate(port_s_in);
//     server_out.set_channel(chan_s.clone());

//     port_s_out.set_next_gate(server_in);

//     cx.create_channel(chan_a);
//     cx.create_channel(chan_b);
//     cx.create_channel(chan_c);
//     cx.create_channel(chan_s);

//     cx.create_module(client_a);
//     cx.create_module(client_b);
//     cx.create_module(client_c);

//     cx.create_module(router);

//     cx.create_module(server);

//     let rt = Runtime::new(rt);
//     let _ = rt.run();
// }

#[NdlModule]
struct HookDestructionModule {
    handle: Option<HookHandle>,
}

impl Module for HookDestructionModule {
    fn new() -> Self {
        Self { handle: None }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        self.handle = Some(create_hook(
            PeriodicHook::new(|_| {}, Duration::from_secs(12), ()),
            1,
        ));

        schedule_in(Message::new().build(), Duration::from_secs(1000));
    }

    fn handle_message(&mut self, _msg: Message) {
        if let Some(handle) = self.handle.take() {
            println!("{:?}", handle);
            destroy_hook(handle)
        }
    }
}

#[test]
#[serial]
fn hook_destruction() {
    let mut rt = NetworkRuntime::new(());
    let mut cx = BuildContext::new(&mut rt);
    let m = HookDestructionModule::build_named(ObjectPath::root_module("Root"), &mut cx);
    cx.create_module(m);

    let rt = Runtime::new(rt);
    let _ = rt.run().unwrap();
}
