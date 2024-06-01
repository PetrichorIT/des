//!
//! Tools for building a module/net oriented simulation.
//!

mod par;
mod path;
mod runtime;
mod util;

pub mod channel;
pub mod gate;
pub mod message;
pub mod module;
pub mod processing;
pub mod topology;

pub(crate) use self::runtime::HandleMessageEvent;
pub(crate) use self::runtime::MessageExitingConnection;
pub(crate) use self::runtime::NetEvents;

pub use self::par::*;
pub use self::path::*;
pub use self::runtime::*;

cfg_async! {
    use  tokio::task::{JoinSet, JoinHandle};

    /// Joins a future sync
    pub fn join(handle: JoinHandle<()>) {
        let mut set = JoinSet::new();
        set.spawn(async move {
            handle.await.unwrap();
        });
        if let Some(result) = set.try_join_next() {
            match result {
                Ok(()) => {},
                Err(e) if e.is_panic() => panic!("{e}"),
                Err(_) => {}
            }
        } else {
            panic!("Failed to join task");
        }
    }
}
