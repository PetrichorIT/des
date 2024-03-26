#![cfg(feature = "ndl")]

use std::sync::atomic::{AtomicUsize, Ordering};

use des::{prelude::*, registry};
use des_ndl::error::RootResult;

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
            self.dst = par("dst")
                .as_option()
                .map(|s| s.parse::<usize>().unwrap())
                .unwrap_or(0);
            self.rem = par("c")
                .as_option()
                .map(|s| s.parse::<usize>().unwrap())
                .unwrap_or(0);
            self.delay = Duration::from_secs_f64(
                par("delay")
                    .as_option()
                    .map(|s| s.parse::<f64>().unwrap())
                    .unwrap_or(1.0),
            );

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

        fn at_sim_end(&mut self) {
            if let Some(v) = par("expected")
                .as_option()
                .map(|v| v.parse::<usize>().unwrap())
            {
                assert_eq!(v, self.rcv, "failed at module: {}", current().path());
            }
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
use common::*;
use serial_test::serial;

#[test]
#[serial]
fn small_network() -> RootResult<()> {
    // Logger::new().set_logger();

    let mut app = Sim::ndl(
        "tests/ndl/small_network/main.ndl",
        registry![Main, Node, Router, Debugger],
    )?;
    app.include_par_file("tests/ndl/small_network/main.par")
        .unwrap();

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
fn ring_topology() -> RootResult<()> {
    // Logger::new().set_logger();

    let mut app = Sim::ndl(
        "tests/ndl/ring_topo/main.ndl",
        registry![Main, Node, Router, Debugger],
    )?;
    app.include_par_file("tests/ndl/ring_topo/main.par")
        .unwrap();

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
        assert!(par("addr").is_some());
        Self
    }
}

impl Module for Single {}

#[test]
#[serial]
fn build_with_preexisting_sim() -> RootResult<()> {
    let mut sim = Sim::new(());
    sim.include_par("alice.addr = 1.1.1.1\n");
    sim.build_ndl("tests/ndl/single.ndl", registry![Single, else _])?;

    let _ = Builder::seeded(123).build(sim).run();
    Ok(())
}

#[test]
#[serial]
fn non_std_gate_connections() -> RootResult<()> {
    let sim = Sim::ndl(
        "tests/ndl/local-con.ndl",
        Registry::new().with_default_fallback(),
    )?;
    let _ = Builder::seeded(123).build(sim).run();
    Ok(())
}

#[test]
#[serial]
fn registry_missing_symbol() {
    let sim: Result<Sim<()>, des_ndl::error::RootError> = Sim::ndl(
        "tests/ndl/ab-deep.ndl",
        Registry::new().symbol("Main", |_| Debugger),
    );
    let errors = sim.unwrap_err().errors;
    assert_eq!(errors.len(), 3);
    assert_eq!(
        errors.get(0).unwrap().internal.to_string(),
        "symbol 'A' at 'a' could not be resolved by the registry",
    );
    assert_eq!(
        errors.get(1).unwrap().internal.to_string(),
        "symbol 'B' at 'a.b' could not be resolved by the registry",
    );
    assert_eq!(
        errors.get(2).unwrap().internal.to_string(),
        "symbol 'B' at 'b' could not be resolved by the registry",
    );
}

#[test]
#[serial]
fn registry_fmt() {
    assert_eq!(
        format!("{:?}", Registry::new().symbol("A", |_| Debugger)),
        "Registry"
    );
}

#[test]
#[serial]
fn registry_custom_resolver() -> RootResult<()> {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let registry = Registry::default()
        .symbol("A", |_| Debugger)
        .symbol("Main", |_| Debugger)
        .custom(|_, symbol| {
            if symbol == "B" {
                COUNTER.fetch_add(1, Ordering::SeqCst);
                Some(Debugger)
            } else {
                None
            }
        });

    let sim = Sim::ndl("tests/ndl/ab.ndl", registry)?;
    let _ = Builder::seeded(123).build(sim).run();

    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

    Ok(())
}

struct Sender;
impl Module for Sender {
    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().build(), "port")
    }
}

#[test]
#[serial]
fn registry_default_fallback_does_not_pani() -> RootResult<()> {
    let registry = Registry::default()
        .symbol("A", |_| Sender)
        .with_default_fallback();

    let sim = Sim::ndl("tests/ndl/ab.ndl", registry)?;
    let _ = Builder::seeded(123).build(sim).run();

    Ok(())
}
