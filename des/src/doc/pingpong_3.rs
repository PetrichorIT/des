//! Ping-Pong as an asynchronous network-simulation with TCP socket
//!
//!
//!
//! ```
//! use des::prelude::*;
//! use des::registry;
//! use inet::interface::*;
//!
//! struct Ping;
//!
//! 
//! impl AsyncModule for Ping {
//!     fn new() -> Ping {
//!         Ping
//!     }
//!
//!     async fn at_sim_start(&mut self, _: usize) {
//!         add_interface(
//!             Interface::ethv4(
//!                 NetworkDevice::eth(),
//!                 Ipv4Addr::new(209, 0, 3, 103)
//!             )
//!         );
//!
//!         let sock = inet::UdpSocket::bind("0.0.0.0:0").await.unwrap();
//!         /* ... */
//!     }
//! }
//!
//! fn main() {
//!     # return;
//!     inet::init();
//!     let app = NdlApplication::new("path/to/ndl", registry![Ping])
//!         .map_err(|e| println!("{e}"))
//!         .unwrap();
//!     let rt = Runtime::new(NetworkApplication::new(app));
//!     let _  = rt.run();
//! }
//! ```
