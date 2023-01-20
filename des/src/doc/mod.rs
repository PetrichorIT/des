//! Guide-level documentation
//!
//! DES offers multiple levels of abstraction to create
//! discrete event-simulations. To illustrate the thoses
//! tools a simple Ping-Pong simulation will be implemented using
//! all three levels of abstraction:
//!
//! - A simple discrete-event-simulation using a custom event set [see here](pingpong_1),
//! - A network-simulation using a generic network layer [see here](pingpong_2),
//! - An asynchronus network-simulation using TCP sockets [see here](pingpong_3).
//!

pub mod pingpong_1;
pub mod pingpong_2;
// pub mod pingpong_3;
