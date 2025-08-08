//! Implementing a Ping-Pong application using the `net` feature of `des`.
//!
//! ## The task
//!
//! The simulation should describe 30 individual 'pings', spaced one second apart, being send to a peer
//! and responded by with a equivalent 'pong'. The number of received pings and pongs should be counted
//! in the global scope.
//!
//! # Requirements
//!
//! This implementation requires the `net` feature, which internally includes network
//! abstractions, macros and serialization implementations. By default this feature does NOT
//! include `tokio`.

use des::prelude::*;
use std::io;

// ## The hosts
//
// Instead of defining events, an event set is already provided by the `net` feature. Buisness logic is written
// in the form of 'modules' which represent independent computing entities in a network (aka hosts). The network
// fabric can be created using the types povided by the `net` feature.
//
// Therefore we will create two modules `Pinger` and `Ponger` that send messages to each other
// over a connecting fabric. Additionally the `Pinger` will send itself an `INTERVAL` message to wake up once pre second
// to send the next `PING`.

struct Pinger {
    pongs_received: usize,
}

struct Ponger {
    pings_received: usize,
}

// ## The network
//
// In this example, the network creation function is up here to illustrate the setup of our network fabric in advance.
// To create a network simulation use the `Sim` type. This builder can be used to instantiate arbitrary network
// topologies.
//
// A network consists of three parts:
// - Modules / Nodes: which act as the freely defineable computing entities in the fabric
// - Gates: which represent communication ingress/egress points on each module
// - Channels: which connect gates of different modules to connect them.
//
// In this example we create two nodes from our custom types `Pinger` and `Ponger`,
// each with one gate called `port`. Using the handles to the gates we obtained
// we connect the two gates using a channel with some metrics. Note that gates
// can be connected without a channel object, in which case no delay or buffering
// will be applied to the connection.

fn build_network() -> Sim<()> {
    let mut sim = Sim::new(());
    sim.node("pinger", Pinger { pongs_received: 0 });
    sim.node("ponger", Ponger { pings_received: 0 });

    let ping_gate = sim.gate("pinger", "port");
    let pong_gate = sim.gate("ponger", "port");

    let metrics = ChannelMetrics::new(
        8_000_000,
        Duration::from_millis(80),
        Duration::ZERO,
        ChannelDropBehaviour::Drop,
    );
    ping_gate.connect(pong_gate, Some(Channel::new(metrics)));

    sim.freeze()
}

// ## Messages
//
// All messages in the fabric are of the type `Message`. This type can encapusalted generic message contents even without the ability
// to serialize or to clone. To distinguish these messages, message kinds can be used.
//
// In this example, we have three kinds of messages:

const INTERVAL: MessageKind = 0;
const PING: MessageKind = 1;
const PONG: MessageKind = 2;

// ## The `Module` trait
//
// To implement custom logic, all modules must implement the `Module` trait. This trait provides various APIs to interact
// with the modules lifecycle and communication behaviour, but the most important are the following:
//
// - `at_sim_start`: A function that is called when the simulation is started to instantiate arbitary local data or processes.
// - `handle_message`: A function that is called whenever the module receives a message.
// - `at_sim_end`: A function that is called when the simulation is shutting down
//
// In this example we use `at_sim_start` to create the inital `INTERVAL` event. Using the `schedule_*` functions a module
// can send itself a message with a delay without using the fabric at all. This can be used to implement timers, wakers
// or other internal logic. In this case we use it to send the `INTERVAL` event 30 times to schedule each required `PING`.
//
// The `handle_message` method for the `Pinger` should only receive two kinds of events: `INTERVAL` and `PONG`. Each
// event is handled in turn, either sending the required `PING` message onto the created communication port `port`
// using the `send*` API, or just counting the received `PONG`s.
//
// At last the `at_sim_end` function checks whether the simulation ran as expected. Runtime Errors
// can be returned using this function but just calling assertions is also ok,
// since panics will be caught and returned as a runtime error.

impl Module for Pinger {
    fn at_sim_start(&mut self, _stage: usize) {
        for i in 0..30 {
            schedule_in(Message::default().kind(INTERVAL), Duration::from_secs(i));
        }
    }

    fn handle_message(&mut self, msg: Message) {
        match msg.header().kind {
            INTERVAL => {
                send(Message::default().kind(PING), "port");
            }
            PONG => {
                self.pongs_received += 1;
            }
            _ => panic!("unexpeced"),
        }
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        if self.pongs_received == 30 {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Invalid number of PONGs").into())
        }
    }
}

// The implementation of `Ponger` is equivalent, in this case using assertions
// instead of explicit errors.

impl Module for Ponger {
    fn handle_message(&mut self, msg: Message) {
        match msg.header().kind {
            PING => {
                self.pings_received += 1;
                send(Message::default().kind(PONG), "port");
            }
            _ => panic!("unexpeced"),
        }
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        assert_eq!(self.pings_received, 30);
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
