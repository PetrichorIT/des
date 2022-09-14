use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;

use des::prelude::*;
use des::stats::ProfilerOutputTarget;
use des::tokio;
use des::tokio::io::AsyncReadExt;
use des::tokio::io::AsyncWriteExt;
use des::tokio::net::IOContext;
use des::tokio::net::TcpListener;
use des::tokio::net::TcpStream;

const REQ_STR: [&'static [u8]; 5] = [b"100b", b"1k", b"10k", b"100k", b"1mb"];

#[NdlModule("examples/net")]
struct Client {}

#[async_trait::async_trait]
impl AsyncModule for Client {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        let ip = par("ip").unwrap().parse::<Ipv4Addr>().unwrap();
        IOContext::new(random::<[u8; 6]>(), ip).set();

        // if ip.octets()[3] != 120 {
        //     return;
        // }

        send(
            Message::new().kind(10).content(IpAddr::V4(ip)).build(),
            "out",
        );

        tokio::spawn(async move {
            let delay = random::<f64>() * 10.0;
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;

            let mut sock = TcpStream::connect("212.71.90.69:8000").await.unwrap();

            for _ in 0..3 {
                let i = random::<usize>() % 5;
                sock.write_all(&REQ_STR[i]).await.unwrap();

                let mut header = [0u8; 4];
                sock.read_exact(&mut header).await.unwrap();

                let size = u32::from_be_bytes(header) as usize;
                let mut buf = vec![0; size];
                sock.read_exact(&mut buf).await.unwrap();

                sock.write_all(b"DONE").await.unwrap();
            }

            let delay = random::<f64>() * 10.0 + 5.0;
            shutdow_and_restart_at(SimTime::now() + delay)
        });
    }
}

#[NdlModule("examples/net")]
struct Server {}

#[async_trait::async_trait]
impl AsyncModule for Server {
    fn new() -> Self {
        Self {}
    }

    async fn at_sim_start(&mut self, _: usize) {
        let ip = par("ip").unwrap().parse::<Ipv4Addr>().unwrap();
        IOContext::new(random::<[u8; 6]>(), ip).set();

        send(
            Message::new().kind(10).content(IpAddr::V4(ip)).build(),
            "out",
        );

        tokio::spawn(async {
            let sock = TcpListener::bind("0.0.0.0:8000").await.unwrap();
            while let Ok((mut stream, _)) = sock.accept().await {
                tokio::spawn(async move {
                    for _ in 0..3 {
                        let mut req = [0u8; 32];
                        let n = stream.read(&mut req).await.unwrap();
                        match &req[..n] {
                            b"100b" => {
                                let value = [42u8; 10];
                                stream.write_u32(value.len() as u32).await.unwrap();
                                stream.write_all(&value).await.unwrap();

                                let mut ack = [0u8; 4];
                                stream.read_exact(&mut ack).await.unwrap();
                                assert_eq!(b"DONE", &ack);
                            }
                            b"1k" => {
                                let value = [56u8; 10];
                                stream.write_u32(value.len() as u32).await.unwrap();
                                stream.write_all(&value).await.unwrap();

                                let mut ack = [0u8; 4];
                                stream.read_exact(&mut ack).await.unwrap();
                                assert_eq!(b"DONE", &ack);
                            }
                            b"10k" => {
                                let value = [22u8; 10];
                                stream.write_u32(value.len() as u32).await.unwrap();
                                stream.write_all(&value).await.unwrap();

                                let mut ack = [0u8; 4];
                                stream.read_exact(&mut ack).await.unwrap();
                                assert_eq!(b"DONE", &ack);
                            }
                            b"100k" => {
                                let value = [40u8; 10];
                                stream.write_u32(value.len() as u32).await.unwrap();
                                stream.write_all(&value).await.unwrap();

                                let mut ack = [0u8; 4];
                                stream.read_exact(&mut ack).await.unwrap();
                                assert_eq!(b"DONE", &ack);
                            }
                            b"1mb" => {
                                let value = [44u8; 10];
                                stream.write_u32(value.len() as u32).await.unwrap();
                                stream.write_all(&value).await.unwrap();

                                let mut ack = [0u8; 4];
                                stream.read_exact(&mut ack).await.unwrap();
                                assert_eq!(b"DONE", &ack);
                            }
                            _ => todo!(),
                        }
                    }
                });
            }
        });
    }
}

#[NdlModule("examples/net")]
struct Router {
    fwd: HashMap<IpAddr, GateRef>,
}

impl Module for Router {
    fn new() -> Self {
        Self {
            fwd: HashMap::new(),
        }
    }

    fn handle_message(&mut self, msg: Message) {
        // println!("{:?}", msg);
        match msg.header().kind {
            10 => {
                let (addr, meta) = msg.cast::<IpAddr>();
                let last_gate = meta.last_gate.as_ref().unwrap();
                let last_gate = match last_gate.name() {
                    "in" => gate("out", last_gate.pos()),
                    "in_server" => gate("out_server", 0),
                    _ => unreachable!(),
                }
                .unwrap();
                log::info!("Added fwd entry for {} --> {:?}", addr, last_gate.str());
                self.fwd.insert(addr, last_gate);
            }
            _ => {
                let dest = msg.header().dest_addr.ip();
                let gate = self.fwd.get(&dest).unwrap().clone();
                send(msg, gate)
            }
        }
    }
}

#[NdlSubsystem("examples/net")]
#[derive(Debug, Default)]
struct Main {}

fn main() {
    // ScopedLogger::new()
    //     .interal_max_log_level(log::LevelFilter::Info)
    //     .finish()
    //     .unwrap();

    let app = Main::default().build_rt();
    let rt = Runtime::new_with(
        app,
        RuntimeOptions::seeded(123).max_time(SimTime::from_duration(Duration::from_secs(10000))),
    );

    let (_, _, profiler, _) = rt.run().unwrap_premature_abort();
    profiler
        .write_to(
            ProfilerOutputTarget::new()
                .write_into("net.output")
                .write_event_count_into("net.event_count.json"),
        )
        .unwrap();
    // profiler.write_to("net.out.json").unwrap()
}
