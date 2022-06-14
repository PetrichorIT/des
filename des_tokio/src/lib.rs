//! A modifed reexport of tokio for a simulation context.

// DOES NOT NEED EXPORT, OS SPECIFIC APIs ARE IRRELEVANT
/* pub use tokio::doc; */

// Filesystem export could be useful for reading configs
// writing logs and the like.
pub use tokio::fs;

// IO should be exported either way.
pub use tokio::io;

// Net should not be used since it relies on primitves out
// side of simulation control. Also namecollision with des::net
/* pub use tokio::net; */

// Process could be used to invoke subprocedures to complex
// to implement locally.
pub use tokio::process;

// Signal has no use whatsoever since it signals to the
// simulation process != signal to module specific runtime.
/* pub use tokio::signal; */

// Does only provied traits but is does not hurt
pub use tokio::stream;

// Custom impls for channels needed to build a meaningful runtime.
pub mod sync;

// Task spawning and management is quiet useful.
pub use tokio::task;

// Time will not be exported since Sleep / Timeout / Interval are forbidden
// and time primitves are implemented under des::time
/* pub use tokio::time; */

pub use tokio::join;
pub use tokio::pin;
pub use tokio::select;
pub use tokio::task_local;
pub use tokio::try_join;

pub use tokio::spawn;

pub use tokio::main;
pub use tokio::test;
