use common::*;
use des::{prelude::*, registry};
use des_net_utils::ndl::error;
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};

#[path = "common/mock.rs"]
mod mock;

mod common {
    use des::prelude::*;

    #[derive(Default)]
    pub struct Main;
    impl Module for Main {}

    #[derive(Default)]
    pub struct Node {
        dst: usize,
        rem: usize,
        delay: Duration,
        rcv: usize,
    }
    impl Module for Node {
        fn at_sim_start(&mut self, _stage: usize) {
            self.dst = current().prop::<usize>("dst").unwrap().get();
            self.rem = current().prop::<usize>("c").unwrap().get();
            self.delay =
                Duration::from_secs_f64(match current().prop::<f64>("delay").unwrap().get() {
                    0.0 => 1.0,
                    other => other,
                });

            tracing::info!(
                "sim_start(dst := {}, c := {}, delay := {})",
                self.dst,
                self.rem,
                self.delay.as_secs_f64()
            );
            if self.rem > 0 {
                schedule_in(Message::new().kind(1).build(), self.delay)
            }
        }

        fn handle_message(&mut self, msg: Message) {
            match msg.header().kind {
                1 => {
                    self.rem -= 1;
                    send(Message::new().kind(2).id(self.dst as u16).build(), "out");

                    if self.rem > 0 {
                        schedule_in(Message::new().kind(1).build(), self.delay)
                    }
                }
                2 => {
                    if current().name().starts_with("node") {
                        assert_eq!(format!("node[{}]", msg.header().id), current().name());
                        self.rcv += 1;
                    }
                    if current().name().starts_with("ring") {
                        if format!("ring[{}]", msg.header().id) == current().name() {
                            self.rcv += 1;
                        } else {
                            send(msg, "out")
                        }
                    }
                }
                _ => unreachable!(),
            }
        }

        fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
            let v = current().prop::<usize>("expected").unwrap().get();
            assert_eq!(v, self.rcv, "failed at module: {}", current().path());

            Ok(())
        }
    }

    #[derive(Default)]
    pub struct Debugger;
    impl Module for Debugger {}

    #[derive(Default)]
    pub struct Router;
    impl Module for Router {
        fn handle_message(&mut self, msg: Message) {
            let g = current().gate("out", msg.header().id as usize).unwrap();
            send(msg, g);
        }
    }
}

#[test]
#[serial]
fn small_network() -> Result<(), Box<dyn std::error::Error>> {
    // Logger::new().set_logger();

    let app = Sim::new(())
        .with_cfg(include_str!("ndl/small_network/main.par.yml"))
        .with_ndl(
            "tests/ndl/small_network/main.yml",
            registry![Main, Node, Router, Debugger],
        )?;

    let r = Builder::seeded(123)
        .max_time(1000.0.into())
        .build(app)
        .run()
        .unwrap();

    assert_eq!(r.1.as_secs(), 200);
    Ok(())
}

#[test]
#[serial]
fn ring_topology() -> Result<(), Box<dyn std::error::Error>> {
    // Logger::new().set_logger();

    let app = Sim::new(())
        .with_cfg(include_str!("ndl/ring_topo/main.par.yml"))
        .with_ndl(
            "tests/ndl/ring_topo/main.yml",
            registry![Main, Node, Router, Debugger],
        )?;

    let r = Builder::seeded(123)
        .max_time(1000.0.into())
        .build(app)
        .run()
        .unwrap();

    assert_eq!(r.1.as_secs(), 200);
    Ok(())
}

struct Single;

impl RegistryCreatable for Single {
    fn create(path: &ObjectPath, _: &str) -> Self {
        println!("{path}");
        assert!(current()
            .prop::<Option<IpAddr>>("addr")
            .unwrap()
            .get()
            .is_some());
        Self
    }
}

impl Module for Single {}

#[test]
#[serial]
fn build_with_preexisting_sim() -> Result<(), Box<dyn std::error::Error>> {
    let mut sim = Sim::new(());
    sim.include_cfg("alice.addr: 1.1.1.1\n");
    sim = sim.with_ndl("tests/ndl/single.yml", registry![Single, else _])?;

    let _ = Builder::seeded(123).build(sim).run();
    Ok(())
}

#[test]
#[serial]
fn non_std_gate_connections() -> Result<(), Box<dyn std::error::Error>> {
    let sim = Sim::ndl(
        "tests/ndl/local-con.yml",
        Registry::new().with_default_fallback(),
    )?;
    let _ = Builder::seeded(123).build(sim).run();
    Ok(())
}

#[test]
#[serial]
fn registry_missing_symbol() {
    let sim: Result<Sim<()>, error::Error> = Sim::ndl(
        "tests/ndl/ab-deep.yml",
        Registry::new().symbol::<Debugger>("Main"),
    );
    let error = sim.unwrap_err();
    assert_eq!(
        error,
        error::ErrorKind::MissingRegistrySymbol("b".to_string(), "B".to_string())
    );
}

#[test]
#[serial]
fn registry_fmt() {
    assert_eq!(
        format!("{:?}", Registry::new().symbol::<Debugger>("A")),
        "Registry"
    );
}

#[test]
#[serial]
fn registry_custom_resolver() -> Result<(), Box<dyn std::error::Error>> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let registry = Registry::new()
        .symbol::<Debugger>("A")
        .symbol::<Debugger>("Main")
        .symbol_fn("B", |_| {
            COUNTER.fetch_add(1, Ordering::SeqCst);
            Debugger
        });

    let sim = Sim::ndl("tests/ndl/ab.yml", registry)?;
    let _ = Builder::seeded(123).build(sim).run();

    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

    Ok(())
}

#[derive(Debug, Default)]
struct Sender;
impl Module for Sender {
    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().build(), "port")
    }
}

#[test]
#[serial]
fn registry_default_fallback_does_not_panic() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Registry::new()
        .symbol::<Sender>("A")
        .with_default_fallback();

    let sim = Sim::ndl("tests/ndl/ab.yml", registry)?;
    let _ = Builder::seeded(123).build(sim).run();

    Ok(())
}

#[test]
#[serial]
#[should_panic = "cannot add another layer, the registry was finalized by the previous one"]
fn registry_add_layer_after_fallback() {
    let _ = Registry::new()
        .with_default_fallback()
        .symbol::<Sender>("A");
}
