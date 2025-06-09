//! Implementing a Ping-Pong application using the `net` and the `async` feature of `des`.
//!
//! ## The task
//!
//! The simulation should describe 30 individual 'pings', spaced one second apart, being send to a peer
//! and responded by with a equivalent 'pong'. The number of received pings and pongs should be counted
//! in the global scope.
//!
//! # Requirements
//!
//! This implementation requires the `net` and `async` feature, which internally includes network
//! abstractions, macros and serialization implementations. By default this feature does
//! include `tokio` as a dependency.

use des::{
    net::blocks::{self, ModuleBlock},
    prelude::*,
    time,
};

// ## Why async?
//
// While the `net` feature provides ample network abstractions, it still requires
// the simulations to be written in an event based, non-imperative manner. While this can be
// useful, most modern code is written imperativly. The `async` feature bridges that gap.

const PING: MessageKind = 1;
const PONG: MessageKind = 2;

// The feature `async` adds a tokio runtime to each module, so that modules can call
// `tokio::spawn` to spawn async tasks. Alternativly use the `AsyncFn` wrapper to create
// a quasai tokio-main function.
//
// Therefore we create the `Ponger` using a async closure that receives a tokio channel receiver to
// receive incoming messages. Within this closure async-await can be used as expected, as long as the
// relevant APIs support the simulation.
//
// The following APIs can be used:
// - tokio::sync
// - des::time
//
// APIs like `tokio::time` or `tokio::net` cannot be used, because these types are working with real OS sockets or clocks
// not simulated ones in the simulation fabric.

fn ponger() -> impl ModuleBlock {
    blocks::AsyncFn::new(|mut rx| async move {
        let mut pongs_received = 0;
        while let Some(msg) = rx.recv().await {
            assert_eq!(msg.header().kind, PING);
            pongs_received += 1;
            send(Message::default().kind(PONG), "port");
        }

        assert_eq!(pongs_received, 30);
    })
}

// The function to build the network fabric is almost identical
// to the `net` example, since the async feature does not introduce additional requirements.

fn build_network() -> Sim<()> {
    let mut sim = Sim::new(());
    sim.node("pinger", Pinger { pongs_received: 0 });
    sim.node("ponger", ponger());

    let ping_gate = sim.gate("pinger", "port");
    let pong_gate = sim.gate("ponger", "port");

    let metrics = ChannelMetrics::new(
        8_000_000,
        Duration::from_millis(80),
        Duration::ZERO,
        ChannelDropBehaviour::Drop,
    );
    ping_gate.connect(pong_gate, Some(Channel::new(metrics)));

    sim
}

// Alternativly, the tokio runtime can also be used in the normal `Module` API.
//
// While there is no `tokio::main` entry point, calls to `tokio::spawn` will still work, since the entire module
// is executed while a runtime is active (aka entered). Each module has its own dedicated runtime.
//
// Tasks can be scheduled to be joined using the `current().join()` API. This call is non-blocking and will try to join the
// task at the end of the simulation, not immideatly. If the task is not joinable, a runtime error will be produced.
//
// This module creates only one task: a sleep loop that creates 30 ping messages.
// The receiving of PONGs is handled in an event-like manner. This example shows, how
// both paradimes can be used in conjunction.

struct Pinger {
    pongs_received: usize,
}

impl Module for Pinger {
    fn at_sim_start(&mut self, _stage: usize) {
        let handle = tokio::spawn(async move {
            for _ in 0..30 {
                send(Message::default().kind(PING), "port");
                time::sleep(Duration::from_secs(1)).await;
            }
        });
        current().join(handle);
    }

    fn handle_message(&mut self, msg: Message) {
        assert_eq!(msg.header().kind, PONG);
        self.pongs_received += 1;
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.pongs_received, 30);
        Ok(())
    }
}

// At last the runtime is created and run using the `Builder`.

fn main() -> Result<(), RuntimeError> {
    let sim = build_network();
    let rt = Builder::new().build(sim);
    let (_, _, _) = rt.run()?;
    Ok(())
}
