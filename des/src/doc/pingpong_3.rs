//! Ping-Pong as a async network-simulation
//!
//! # The task
//!
//! Two nodes `Pinger` and `Ponger` want to communicate with each other
//! using a bidirection channel. `Pinger` sends a total of 30 `Ping`-messages
//! in intervals of 1s. `Ponger` receives these messages and
//! responds with a `Pong`-Message, that 'Pinger' receives. Both
//! `Pinger` and `Ponger` count the number of messages received and send
//! by them.
//!
//! # The async simulation
//!
//! The feature flag `async` enables writing modules in an asyncchronous matter
//! using async/await based on a tokio runtime.
//!
//! The primary addition of this feature flag is the trait [`Module`] that
//! acts as an async alternative to [`Module`]. The feature flag additionally adds
//! a new wrapper [`AsyncFn`] that enables less boilderplate when creating async modules.
//!
//! ## A simple implemention using `AsyncFn`
//!
//! [`AsyncFn`] is a helper type that wraps a closure, taking in the recieving end
//! of an mpsc channel and returning a future. This channel will be to transmit incoming
//! messages instead of calling `handle_message` on some type T.
//!
//! Additionally we will use the time primitives defined in [`des::time`] to simplifiy
//! interval managment. These time primitives mirror `tokio::time` but are bound to the
//! simulation clock instead of the OS clock.
//!
//! ```
//! use des::prelude::*;
//! use des::net::AsyncFn;
//!
//! const PING: MessageKind = 42;
//! const PONG: MessageKind = 43;
//!
//! fn main() {
//!     let mut sim = Sim::new(());
//!     sim.node("pinger", AsyncFn::new(|mut rx| async move {
//!         tokio::spawn(async {
//!             let mut interval = des::time::interval(Duration::from_secs(1));
//!             for _ in 0..32 {
//!                 interval.tick().await;
//!                 send(Message::default().kind(PING), "to-pong");
//!             }
//!         });
//!
//!         for _ in 0..32 {
//!             let msg = rx.recv().await.unwrap();
//!             assert_eq!(msg.header().kind, PONG);
//!         }
//!     }).require_join());
//!
//!     sim.node("ponger", AsyncFn::new(|mut rx| async move {
//!         while let Some(msg) = rx.recv().await {
//!             send(Message::default().kind(PONG), "to-ping");
//!         }
//!     }));
//!
//!     let to_pong = sim.gate("pinger", "to-pong");
//!     let to_ping = sim.gate("ponger", "to-ping");
//!
//!     let channel = Channel::new(ChannelMetrics {
//!         bitrate: 8_000_000,
//!         latency: Duration::from_millis(10),
//!         jitter: Duration::ZERO,
//!         drop_behaviour: ChannelDropBehaviour::Drop,
//!     });
//!
//!     to_pong.connect(to_ping, Some(channel));
//!
//!     match Builder::new().build(sim).run() {
//!         /* ... */
//!         # _ => {}
//!     }
//! }
//! ```
//!
//! [`Module`]: crate::net::module::Module
//! [`des::time`]: crate::time
//! [`Sim::include_par`]: crate::net::runtime::Sim::include_par
