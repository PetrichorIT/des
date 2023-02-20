//! Network subsection management.
#![allow(unused)]

use std::any::Any;

mod sref;
pub use sref::*;

mod ctx;
pub use ctx::*;

guid!(
    /// A runtime-unqiue identifier for a module / submodule inheritence tree.
    /// * This type is only available of DES is build with the `"net"` feature.*
    #[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub SubsystemId(u16) = MODULE_ID;
);

/// The functions of a subsystem
pub trait Subsystem: Any {}

impl<T: Any> Subsystem for T {}
