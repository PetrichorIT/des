#![cfg(feature = "ndl")]

use des::{ndl::NdlApplication, prelude::*, registry};
use des_ndl::error::RootResult;

mod common {
    use des::prelude::*;

    pub struct Main;
    impl Module for Main {
        fn new() -> Self {
            Self
        }
    }

    pub struct Node {
        dst: usize,
        rem: usize,
        delay: Duration,
        rcv: usize,
    }
    impl Module for Node {
        fn new() -> Self {
            Self {
                dst: 0,
                rem: 0,
                delay: Duration::new(0, 0),
                rcv: 0,
            }
        }

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

            log::info!(
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
                    if module_name().starts_with("node") {
                        assert_eq!(format!("node[{}]", msg.header().id), module_name());
                        self.rcv += 1;
                    }
                    if module_name().starts_with("ring") {
                        if format!("ring[{}]", msg.header().id) == module_name() {
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
                assert_eq!(v, self.rcv, "failed at module: {}", module_path());
            }
        }
    }

    pub struct Debugger;
    impl Module for Debugger {
        fn new() -> Self {
            Self
        }
    }

    pub struct Router;
    impl Module for Router {
        fn new() -> Self {
            Self
        }

        fn handle_message(&mut self, msg: Message) {
            let g = gate("out", msg.header().id as usize).unwrap();
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

    let ndl = NdlApplication::new(
        "tests/ndl/small_network/main.ndl",
        registry![Main, Node, Router, Debugger],
    )?;
    let mut app = NetworkRuntime::new(ndl);
    app.include_par_file("tests/ndl/small_network/main.par");

    let r = Runtime::new_with(app, RuntimeOptions::seeded(123).max_time(1000.0.into()))
        .run()
        .unwrap();

    assert_eq!(r.1.as_secs(), 200);
    Ok(())
}

#[test]
#[serial]
fn ring_topology() -> RootResult<()> {
    // Logger::new().set_logger();

    let ndl = NdlApplication::new(
        "tests/ndl/ring_topo/main.ndl",
        registry![Main, Node, Router, Debugger],
    )?;
    let mut app = NetworkRuntime::new(ndl);
    app.include_par_file("tests/ndl/ring_topo/main.par");

    let r = Runtime::new_with(app, RuntimeOptions::seeded(123).max_time(1000.0.into()))
        .run()
        .unwrap();

    assert_eq!(r.1.as_secs(), 200);
    Ok(())
}
