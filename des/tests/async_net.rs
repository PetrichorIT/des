#![cfg(feature = "async")]
use des::prelude::*;
use des::tokio::sim::net::*;
use serial_test::serial;
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;

#[NdlModule]
struct TcpBindTests {}

#[async_trait::async_trait]
impl AsyncModule for TcpBindTests {
    async fn at_sim_start(&mut self, _: usize) {
        // Set IO Context
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 112)).set();

        // # Test case 1 - Binds
        {
            // Port 8000 done
            let s0 = TcpListener::bind("192.168.2.112:8000").await.unwrap();
            assert_eq!(
                s0.local_addr().unwrap(),
                "192.168.2.112:8000".parse::<SocketAddr>().unwrap()
            );

            let s1 = TcpListener::bind("192.168.2.112:0").await.unwrap();
            assert_eq!(
                s1.local_addr().unwrap(),
                "192.168.2.112:1024".parse::<SocketAddr>().unwrap()
            );

            let s2 = TcpListener::bind("192.168.2.112:0").await.unwrap();
            assert_eq!(
                s2.local_addr().unwrap(),
                "192.168.2.112:1025".parse::<SocketAddr>().unwrap()
            );

            let s3 = TcpListener::bind("0.0.0.0:0").await.unwrap();
            assert_eq!(
                s3.local_addr().unwrap(),
                "192.168.2.112:1026".parse::<SocketAddr>().unwrap()
            );

            // Maual set of 1027
            let s4 = TcpListener::bind("0.0.0.0:1027").await.unwrap();
            assert_eq!(
                s4.local_addr().unwrap(),
                "192.168.2.112:1027".parse::<SocketAddr>().unwrap()
            );

            // auto assign should catch that
            let s5 = TcpListener::bind("0.0.0.0:0").await.unwrap();
            assert_eq!(
                s5.local_addr().unwrap(),
                "192.168.2.112:1028".parse::<SocketAddr>().unwrap()
            );

            // keep them alive
            let _ = (s0, s1, s2, s3, s4, s5);
        }

        // # Test case 2 - Bind Drops
        {
            // ptr is 1029
            let s0 = TcpListener::bind("0.0.0.0:0").await.unwrap();
            assert_eq!(
                s0.local_addr().unwrap(),
                "192.168.2.112:1029".parse::<SocketAddr>().unwrap()
            );

            // ptr is 1030
            // manual set of 1030
            let s1 = TcpListener::bind("0.0.0.0:1030").await.unwrap();
            assert_eq!(
                s1.local_addr().unwrap(),
                "192.168.2.112:1030".parse::<SocketAddr>().unwrap()
            );

            // drop so auto assign can reassign port
            drop(s1);

            let s2 = TcpListener::bind("0.0.0.0:0").await.unwrap();
            assert_eq!(
                s2.local_addr().unwrap(),
                "192.168.2.112:1030".parse::<SocketAddr>().unwrap()
            );
        }

        // # Test case 3 - No default interface binds
        {
            let s0 = TcpListener::bind("127.0.0.1:0").await.unwrap();
            assert_eq!(
                s0.local_addr().unwrap(),
                "127.0.0.1:1031".parse::<SocketAddr>().unwrap()
            );
        }
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

#[NdlModule]
struct TcpConnectPing {
    handle: Option<JoinHandle<()>>,
}

impl NameableModule for TcpConnectPing {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            handle: None,
        }
    }
}

#[async_trait::async_trait]
impl AsyncModule for TcpConnectPing {
    async fn at_sim_start(&mut self, _: usize) {
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 110)).set();
        self.handle = Some(tokio::spawn(async {
            // First get a real connection;
            println!("Case #1");
            let s0 = TcpStream::connect("192.168.2.112:8000").await.unwrap();
            assert_eq!(
                s0.local_addr().unwrap(),
                "192.168.2.110:1024".parse::<SocketAddr>().unwrap()
            );
            drop(s0);

            // Get a IP based timeout
            println!("Case #2");
            let s1 = TcpStream::connect("192.168.2.99:8000").await.unwrap_err();
            assert_eq!(s1.kind(), ErrorKind::NotConnected);

            // get a port based timeout
            println!("Case #3");
            let s2 = TcpStream::connect("192.168.2.112:8001").await.unwrap_err();
            assert_eq!(s2.kind(), ErrorKind::NotConnected);

            // get a connection from a list of addrs.
            println!("Case #4");
            let s3 = TcpStream::connect(
                &[
                    "192.168.2.99:8000".parse::<SocketAddr>().unwrap(),
                    "192.168.2.112:8001".parse::<SocketAddr>().unwrap(),
                    "192.168.2.112:8000".parse::<SocketAddr>().unwrap(),
                ][..],
            )
            .await
            .unwrap();
            assert_eq!(
                s3.local_addr().unwrap(),
                "192.168.2.110:1029".parse::<SocketAddr>().unwrap()
            );
            assert_eq!(
                s3.peer_addr().unwrap(),
                "192.168.2.112:8000".parse::<SocketAddr>().unwrap()
            );
            drop(s3);
        }));
    }

    async fn at_sim_end(&mut self) {
        self.handle.take().unwrap().await.unwrap();
    }

    async fn handle_message(&mut self, _: Message) {}
}

#[NdlModule]
struct TcpConnectPong {
    handle: Option<JoinHandle<()>>,
}

impl NameableModule for TcpConnectPong {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            handle: None,
        }
    }
}

#[async_trait::async_trait]
impl AsyncModule for TcpConnectPong {
    async fn at_sim_start(&mut self, _: usize) {
        self.handle = Some(tokio::spawn(async {
            IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 112)).set();

            let server = TcpListener::bind("0.0.0.0:8000").await.unwrap();
            // expecting 2 valid connections
            for _ in 0..2 {
                let (stream, addr) = server.accept().await.unwrap();
                assert_eq!(addr.ip(), Ipv4Addr::new(192, 168, 2, 110));
                println!("Got connection from {:?}", addr);
                drop(stream)
            }
        }));
    }

    async fn at_sim_end(&mut self) {
        self.handle.take().unwrap().await.unwrap();
    }

    async fn handle_message(&mut self, _: Message) {}
}

#[test]
#[serial]
fn tcp_connection_buildup() {
    let mut rt = NetworkRuntime::new(());
    let mut ping = TcpConnectPing::named_root(ModuleCore::new_with(
        ObjectPath::root_module("ping".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let ping_in = ping.create_gate("in", GateServiceType::Input, &mut rt);
    let mut ping_out = ping.create_gate("out", GateServiceType::Output, &mut rt);

    let mut pong = TcpConnectPong::named_root(ModuleCore::new_with(
        ObjectPath::root_module("pang".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let pong_in = pong.create_gate("in", GateServiceType::Input, &mut rt);
    let mut pong_out = pong.create_gate("out", GateServiceType::Output, &mut rt);

    ping_out.set_next_gate(pong_in);
    pong_out.set_next_gate(ping_in);

    let ping_to_pong = Channel::new(
        ObjectPath::channel_with("to_pong", ping.path()),
        ChannelMetrics::new(1000, Duration::from_millis(100), Duration::ZERO),
    );

    let pong_to_ping = Channel::new(
        ObjectPath::channel_with("to_ping", pong.path()),
        ChannelMetrics::new(1000, Duration::from_millis(100), Duration::ZERO),
    );

    ping_out.set_channel(ping_to_pong);
    pong_out.set_channel(pong_to_ping);

    rt.create_module(ping);
    rt.create_module(pong);

    let rt = Runtime::new(rt);
    let _ = rt.run();
}

#[NdlModule]
struct TcpFullPing {
    handle: Option<JoinHandle<()>>,
}

impl NameableModule for TcpFullPing {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            handle: None,
        }
    }
}

#[async_trait::async_trait]
impl AsyncModule for TcpFullPing {
    async fn handle_message(&mut self, _: Message) {}

    async fn at_sim_start(&mut self, _: usize) {
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 100)).set();
        self.handle = Some(tokio::spawn(async {
            let server = TcpListener::bind("0.0.0.0:8000").await.unwrap();

            let (stream, src) = server.accept().await.unwrap();
            let pong_handle = tokio::spawn(async move {
                let mut stream = stream;
                let src = src;

                assert_eq!(src, "192.168.2.110:1024".parse::<SocketAddr>().unwrap());
                let mut buf = [0u8; 1024];

                let n = stream.read(&mut buf).await.unwrap();
                assert_eq!(n, 11);

                let n = stream.read(&mut buf).await.unwrap();
                assert_eq!(n, 2);
            });

            let (stream, src) = server.accept().await.unwrap();
            let pang_handle = tokio::spawn(async move {
                let mut stream = stream;
                let src = src;

                assert_eq!(src, "192.168.2.112:1024".parse::<SocketAddr>().unwrap());
                let mut buf = [0u8; 1024];

                let n = stream.read(&mut buf).await.unwrap();
                assert_eq!(n, 11);

                let n = stream.read(&mut buf).await.unwrap();
                assert_eq!(n, 2);
            });

            pong_handle.await.unwrap();
            pang_handle.await.unwrap();
        }));
    }
    async fn at_sim_end(&mut self) {
        self.handle.take().unwrap().await.unwrap()
    }
}

#[NdlModule]
struct TcpFullPong {
    handle: Option<JoinHandle<()>>,
}

impl NameableModule for TcpFullPong {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            handle: None,
        }
    }
}

#[async_trait::async_trait]
impl AsyncModule for TcpFullPong {
    async fn handle_message(&mut self, _: Message) {}

    async fn at_sim_start(&mut self, _: usize) {
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 110)).set();
        self.handle = Some(tokio::spawn(async {
            let mut sock = TcpStream::connect("192.168.2.100:8000").await.unwrap();

            sock.write_all(b"Hello World").await.unwrap();
            // Sleep is nessecary to prevent over-busy channels
            tokio::sim::time::sleep(Duration::from_millis(900)).await;
            sock.write_all(b"AA").await.unwrap();
        }));
    }
    async fn at_sim_end(&mut self) {
        self.handle.take().unwrap().await.unwrap()
    }
}

#[NdlModule]
struct TcpFullPang {
    handle: Option<JoinHandle<()>>,
}

impl NameableModule for TcpFullPang {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            handle: None,
        }
    }
}

#[async_trait::async_trait]
impl AsyncModule for TcpFullPang {
    async fn handle_message(&mut self, _: Message) {}

    async fn at_sim_start(&mut self, _: usize) {
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 112)).set();

        self.handle = Some(tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let mut sock = TcpStream::connect("192.168.2.100:8000").await.unwrap();

            sock.write_all(b"Hello World").await.unwrap();
            // Sleep is nessecary to prevent over-busy channels
            tokio::sim::time::sleep(Duration::from_millis(200)).await;
            sock.write_all(b"AA").await.unwrap();
        }));
    }
    async fn at_sim_end(&mut self) {
        self.handle.take().unwrap().await.unwrap()
    }
}

#[NdlModule]
struct TcpFullRouter {}

impl Module for TcpFullRouter {
    fn handle_message(&mut self, msg: Message) {
        if msg.header().typ() == MessageType::Tcp {
            match msg.header().dest_addr.ip() {
                IpAddr::V4(ip) if ip == Ipv4Addr::new(192, 168, 2, 110) => self.send(msg, "out1"),
                IpAddr::V4(ip) if ip == Ipv4Addr::new(192, 168, 2, 112) => self.send(msg, "out2"),
                _ => unreachable!(),
            }
        } else {
            println!("Unknown msg: {:?}", msg)
        }
    }
}

/*
Ping - Server
Pong - Client
Pang - Client

*/
#[test]
#[serial]
fn tcp_full_test() {
    // ScopedLogger::new()
    //     // .interal_max_log_level(log::LevelFilter::Warn)
    //     .finish()
    //     .unwrap();

    let mut rt = NetworkRuntime::new(());
    let mut ping = TcpFullPing::named_root(ModuleCore::new_with(
        ObjectPath::root_module("ping".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let ping_in1 = ping.create_gate("in1", GateServiceType::Input, &mut rt);

    let ping_in2 = ping.create_gate("in2", GateServiceType::Input, &mut rt);

    let mut router = TcpFullRouter::named_root(ModuleCore::new_with(
        ObjectPath::root_module("router".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let mut ping_out = ping.create_gate("out", GateServiceType::Output, &mut rt);
    let ping_router_in = router.create_gate("in", GateServiceType::Input, &mut rt);

    ping_out.set_next_gate(ping_router_in);

    let mut ping_out1 = router.create_gate("out1", GateServiceType::Output, &mut rt);
    let mut ping_out2 = router.create_gate("out2", GateServiceType::Output, &mut rt);

    let mut pong = TcpFullPong::named_root(ModuleCore::new_with(
        ObjectPath::root_module("pong".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let pong_in = pong.create_gate("in", GateServiceType::Input, &mut rt);
    let mut pong_out = pong.create_gate("out", GateServiceType::Output, &mut rt);

    let mut pang = TcpFullPang::named_root(ModuleCore::new_with(
        ObjectPath::root_module("pang".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));
    let pang_in = pang.create_gate("in", GateServiceType::Input, &mut rt);
    let mut pang_out = pang.create_gate("out", GateServiceType::Output, &mut rt);

    // ping pong connections
    ping_out1.set_next_gate(pong_in);
    pong_out.set_next_gate(ping_in1);

    // ping pang
    ping_out2.set_next_gate(pang_in);
    pang_out.set_next_gate(ping_in2);

    // Channels

    let ping_to_pong = Channel::new(
        ObjectPath::channel_with("to_pong", ping.path()),
        ChannelMetrics::new(1000, Duration::from_millis(100), Duration::ZERO),
    );

    let pong_to_ping = Channel::new(
        ObjectPath::channel_with("to_ping_from_pong", pong.path()),
        ChannelMetrics::new(1000, Duration::from_millis(100), Duration::ZERO),
    );

    ping_out1.set_channel(ping_to_pong);
    pong_out.set_channel(pong_to_ping);

    let ping_to_pang = Channel::new(
        ObjectPath::channel_with("to_pang", ping.path()),
        ChannelMetrics::new(1000000, Duration::from_millis(10), Duration::ZERO),
    );

    let pang_to_ping = Channel::new(
        ObjectPath::channel_with("to_ping_from_pang", pong.path()),
        ChannelMetrics::new(1000000, Duration::from_millis(10), Duration::ZERO),
    );

    ping_out2.set_channel(ping_to_pang);
    pang_out.set_channel(pang_to_ping);

    rt.create_module(ping);
    rt.create_module(pong);
    rt.create_module(pang);

    let rt = Runtime::new(rt);
    let _ = rt.run();
}

#[NdlModule]
struct IoDelayReceiver {}

#[async_trait::async_trait]
impl AsyncModule for IoDelayReceiver {
    async fn at_sim_start(&mut self, _: usize) {
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 10)).set();

        tokio::spawn(async {
            let sock = TcpListener::bind("0.0.0.0:8000").await.unwrap();
            while let Ok((mut stream, _)) = sock.accept().await {
                // NOP
                loop {
                    let mut buf = [0u8; 512];
                    stream.read(&mut buf).await.unwrap();
                }
            }
        });
    }
}

#[NdlModule]
struct IoDelaySender {
    handle: Option<JoinHandle<()>>,
}

impl NameableModule for IoDelaySender {
    fn named(core: ModuleCore) -> Self {
        Self {
            __core: core,
            handle: None,
        }
    }
}

#[async_trait::async_trait]
impl AsyncModule for IoDelaySender {
    async fn at_sim_start(&mut self, _: usize) {
        IOContext::new([1, 2, 3, 4, 5, 6], Ipv4Addr::new(192, 168, 2, 9)).set();

        self.handle = Some(tokio::spawn(async {
            let sock = TcpSocket::new_v4().unwrap();
            sock.set_send_buffer_size(2048).unwrap(); // 2 packets

            let mut sock = sock
                .connect(SocketAddr::from_str("192.168.2.10:8000").unwrap())
                .await
                .unwrap();

            let p512 = [42u8; 512];

            // write 3 packets of 512 bytes
            let t0 = SimTime::now();

            sock.write_all(&p512).await.unwrap();
            sock.write_all(&p512).await.unwrap();
            sock.write_all(&p512).await.unwrap();

            let t1 = SimTime::now();

            assert_eq!(t0, t1);

            // skip to clean all buffers / time
            tokio::time::sleep(Duration::from_secs(1)).await;

            // write 5 packets
            let t0 = SimTime::now();

            sock.write_all(&p512).await.unwrap();
            sock.write_all(&p512).await.unwrap();
            sock.write_all(&p512).await.unwrap();
            sock.write_all(&p512).await.unwrap();
            let t1 = SimTime::now();

            sock.write_all(&p512).await.unwrap();

            let t2 = SimTime::now();
            assert_eq!(t0, t1);

            assert_ne!(t1, t2);
            assert_eq!(t1 + Duration::from_millis(10), t2); // 2 packtes delay of 5ms

            // skip to clean all buffers / time
            tokio::time::sleep(Duration::from_secs(1)).await;

            // write partial packets
            let p1024 = [69u8; 1024];

            let t0 = SimTime::now();

            sock.write_all(&p512).await.unwrap();
            sock.write_all(&p1024).await.unwrap();
            let t1 = SimTime::now();
            sock.write_all(&p1024).await.unwrap();

            let t2 = SimTime::now();
            assert_eq!(t0, t1);

            assert_ne!(t1, t2);
            assert_eq!(t1 + Duration::from_millis(10), t2);
            // 2 packtes delay of 5ms
        }));
    }

    async fn at_sim_end(&mut self) {
        self.handle.take().unwrap().await.unwrap();
    }
}

#[test]
#[serial]
fn delayed_write() {
    let mut rt = NetworkRuntime::new(());
    let mut rx = IoDelayReceiver::named_root(ModuleCore::new_with(
        ObjectPath::root_module("receiver".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let rx_in = rx.create_gate("in", GateServiceType::Input, &mut rt);
    let mut rx_out = rx.create_gate("out", GateServiceType::Output, &mut rt);

    let mut tx = IoDelaySender::named_root(ModuleCore::new_with(
        ObjectPath::root_module("sender".to_string()),
        Ptr::downgrade(&rt.globals()),
    ));

    let tx_in = tx.create_gate("in", GateServiceType::Input, &mut rt);
    let mut tx_out = tx.create_gate("out", GateServiceType::Output, &mut rt);

    rx_out.set_next_gate(tx_in);
    tx_out.set_next_gate(rx_in);

    rt.create_module(rx);
    rt.create_module(tx);

    let rt = Runtime::new(rt);
    let _ = rt.run();
}
