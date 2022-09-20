//! Ping-Pong as an asynchronous network-simulation with TCP socket
//!
//! # The task
//!
//! Two nodes 'Ping' and 'Pong' can communicate with each other
//! using a bidirection channel. 'Ping' sends 30 ping-messages
//! with at an interval of 1s. 'Pong' receives the messages and
//! responds with a pong-message, that 'Ping' receives. Both
//! 'Ping' and 'Pong' count the number of messages received and send
//! by them.
//!
//! # Note
//!
//! This example is based on the simple network-simulation described [here](super::pingpong_2).
//! For information concerning NDL, module binding and constructing consult the previous
//! example.
//!
//! # The simulation
//!
//! This network-simulation is created by using the features `async`.
//!
//! # Async modules
//!
//! When the module behaves asynchronously you can implemente [`AsyncModule`](crate::net::AsyncModule)
//! instead of [`Module`](crate::net::Module) to use async versions of the functions. The semantics
//! remain largly unchanged only with the difference that [`handle_messsage`] is no longer required.
//! When implementing the trait [`AsyncModule`](crate::net::AsyncModule), use [`async_trait`] for
//! more readable function signatures.
//!
//! Based on that lets reimplement 'Pong':
//!
//! ```rust
//! use des::prelude::*;
//! use tokio::{net::*, io::*};
//! use std::net::Ipv4Addr;
//!
//! # #[NdlModule]
//! # struct Pong {}
//! #[async_trait::async_trait]
//! impl AsyncModule for Pong {
//!     fn new() -> Self {
//!         /* ... */
//!         # todo!()
//!     }
//!
//!     async fn at_sim_start(&mut self, _stage: usize) {
//!         IOContext::new([0,0,0,0,0,1], Ipv4Addr::new(192, 168, 2, 100)).set();
//!
//!         tokio::spawn(async {
//!             let sock = TcpListener::bind("0.0.0.0:8000").await.unwrap();
//!             while let Ok((mut stream, from)) = sock.accept().await {
//!                 // Wait for the ping
//!                 let mut buf = [0u8; 1024];
//!                 let n = stream.read(&mut buf).await.unwrap();
//!                 println!("Got '{}'", String::from_utf8_lossy(&buf[..n]));
//!                 // Send the pong
//!                 let pong = b"PONG";
//!                 stream.write_all(pong).await.unwrap();
//!             }
//!         });
//!     }
//! }
//! ```
//!
//! Lets explore this code:
//!
//! At the start, we must define an [`IOContext`](tokio::net::IOContext) on each module so
//! that we can use network primitives provided by tokio. For that we need to define
//! an MAC address and an Ipv4 address. Create *and* set the [`IOContext`] as the first thing
//! in the [`at_sim_start`] function.
//!
//! After that we create our [`TcpListener`] to look for incoming connections on port 8000.
//! Blocking call like [accept] are handled by DES, so that communication over TCP
//! socket can happen without using the [`handle_message`] function. Once a stream
//! was established we wait to receive some data, and the send our pong back.
//! After that the stream is dropped and the next connection can be accepted.
//!
//! Note that our listener was not created directly within the
//! [`at_sim_start`] functoin, but within a extra Tokio-Task. This is nessecary
//! to ensure that all part of the [`at_sim_start`] were fully executed after the
//! start of the simulation. Since our socket we need to life longer that just the simulation
//! start we put it in its own task.
//!
//! After that now lets reimplement 'Ping':
//!
//! ```rust
//! use des::prelude::*;
//! use tokio::{net::*, io::*};
//! use std::net::Ipv4Addr;
//!
//! # #[NdlModule]
//! # struct Ping {}
//! #[async_trait::async_trait]
//! impl AsyncModule for Ping {
//!     fn new() -> Self {
//!         /* ... */
//!         # todo!()
//!     }
//!
//!     async fn at_sim_start(&mut self, _stage: usize) {
//!         IOContext::new([0,0,0,0,0,2], Ipv4Addr::new(192, 168, 2, 200)).set();
//!
//!         tokio::spawn(async {
//!             for _ in 0..30 {
//!                 // Put it in an extra task to keep time in sync
//!                 tokio::spawn(async {
//!                     let mut stream = TcpStream::connect("192.168.2.100:8000").await.unwrap();
//!                     stream.write_all(b"PING").await.unwrap();
//!                     // Wait for pong
//!                     let mut buf = [0u8; 1024];
//!                     let n = stream.read(&mut buf).await.unwrap();
//!                     println!("Got back '{}'", String::from_utf8_lossy(&buf[..n]));
//!                 });
//!                 tokio::time::sleep(Duration::from_secs(1)).await;
//!             }
//!         });
//!     }
//! }
//! ```
//!
//! The rest of the code remains the same. Note that you can use both [`Module`] and [`AsyncModule`]
//! with the feature `async`.
