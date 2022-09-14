// use std::collections::HashMap;

// use crate::net::common::Optional;
// use crate::net::{Message, ParHandle};
// use crate::prelude::{NetworkRuntimeGlobals, ObjectPath, PtrWeakConst};
// use crate::time::{Duration, SimTime};

// pub(crate) enum BufferEvent {
//     Send {
//         msg: Message,
//         time_offset: Duration,
//         out: Box<dyn IntoModuleGate>,
//     },
//     ScheduleIn {
//         msg: Message,
//         time_offset: Duration,
//     },
//     ScheduleAt {
//         msg: Message,
//         time: SimTime,
//     },
//     #[cfg(not(feature = "async-sharedrt"))]
//     Shutdown {
//         restart_at: Option<SimTime>,
//     },
// }

// impl std::fmt::Debug for BufferEvent {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Send {
//                 msg, time_offset, ..
//             } => f
//                 .debug_struct("BufferEvent::Send")
//                 .field("msg", msg)
//                 .field("time_offset", time_offset)
//                 .finish(),
//             Self::ScheduleIn { msg, time_offset } => f
//                 .debug_struct("BufferEvent::ScheduleIn")
//                 .field("msg", msg)
//                 .field("time_offset", time_offset)
//                 .finish(),
//             Self::ScheduleAt { msg, time } => f
//                 .debug_struct("BufferEvent::ScheduleAt")
//                 .field("msg", msg)
//                 .field("time", time)
//                 .finish(),
//             #[cfg(not(feature = "async-sharedrt"))]
//             Self::Shutdown { restart_at } => f
//                 .debug_struct("BufferEvent::Shutdown")
//                 .field("restart_at", restart_at)
//                 .finish(),
//         }
//     }
// }

// // SAFTY:
// // Buffer events can be considered 'Send' since [Message]
// // is 'Send', [Duration] / [SimTime] are primitve and
// // [dyn IntoModuleGate] is either a gate isself or primitve
// // values describing the gate.
// unsafe impl Send for BufferEvent {}

// ///
// /// The asychronous part of a send handle.
// /// This can be Send via threads to enqueu messages
// /// from a module.
// ///
// #[derive(Debug)]
// pub struct SenderHandle {
//     pub(crate) inner: tokio::sync::mpsc::UnboundedSender<BufferEvent>,
//     pub(crate) time_offset: Duration,
//     pub(crate) globals: PtrWeakConst<NetworkRuntimeGlobals>,
//     pub(crate) path: ObjectPath,
// }

// unsafe impl Send for SenderHandle {}

// impl SenderHandle {
//     /// Creates a new handle that points to no module.
//     /// Only fo debuhhing
//     pub fn empty() -> Self {
//         let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
//         drop(rx);
//         Self {
//             inner: tx,
//             time_offset: Duration::ZERO,
//             globals: PtrWeakConst::new(),
//             path: ObjectPath::root_module("empty".to_string()),
//         }
//     }

//     ///
//     /// Adds the duration to the processing time offset.
//     /// All messages send after this time will be delayed by the
//     /// processing time delay.
//     ///
//     pub fn processing_time(&mut self, duration: Duration) {
//         self.time_offset += duration;
//     }

//     ///
//     /// Sends a message onto a given gate. This operation will be performed after
//     /// `handle_message` finished.
//     ///
//     pub fn send(&self, msg: impl Into<Message>, gate: impl IntoModuleGate + 'static) {
//         self.inner
//             .send(BufferEvent::Send {
//                 msg: msg.into(),
//                 time_offset: self.time_offset,
//                 out: Box::new(gate),
//             })
//             .expect("Failed to send to unbounded channel. reciver must have died.");
//     }

//     ///
//     /// Enqueues a event that will trigger the [`Module::handle_message`] function
//     /// in duration seconds, shifted by the processing time delay.
//     ///
//     pub fn schedule_in(&self, msg: impl Into<Message>, duration: Duration) {
//         self.inner
//             .send(BufferEvent::ScheduleIn {
//                 msg: msg.into(),
//                 time_offset: self.time_offset + duration,
//             })
//             .expect("Failed to send to unbounded channel. reciver must have died.");
//     }

//     ///
//     /// Enqueues a event that will trigger the [`Module::handle_message`] function
//     /// at the given `SimTime`
//     ///
//     pub fn schedule_at(&self, msg: impl Into<Message>, time: SimTime) {
//         self.inner
//             .send(BufferEvent::ScheduleAt {
//                 msg: msg.into(),
//                 time,
//             })
//             .expect("Failed to send to unbounded channel. reciver must have died.");
//     }

//     ///
//     /// Shuts down all activity for the module.
//     /// Restarts the module at the given time.
//     ///
//     #[cfg(not(feature = "async-sharedrt"))]
//     pub fn shutdown(&self, restart_at: Option<SimTime>) {
//         assert!(restart_at.unwrap_or(SimTime::MAX) > SimTime::now());

//         self.inner
//             .send(BufferEvent::Shutdown { restart_at })
//             .expect("Failed to send to unbounded channel. reciver must have died.")
//     }

//     ///
//     /// Returns the parameters for the current module.
//     ///
//     #[must_use]
//     pub fn pars(&self) -> HashMap<String, String> {
//         self.globals.parameters.get_def_table(self.path.path())
//     }

//     ///
//     /// Returns a parameter by reference (not parsed).
//     ///
//     #[must_use]
//     pub fn par<'a>(&'a self, key: &'a str) -> ParHandle<'a, Optional> {
//         self.globals.parameters.get_handle(self.path.path(), key)
//     }

//     ///
//     /// Returns a reference to the parameter store, used for constructing
//     /// custom instances of modules.
//     ///
//     #[must_use]
//     pub fn globals(&self) -> PtrWeakConst<NetworkRuntimeGlobals> {
//         PtrWeakConst::clone(&self.globals)
//     }
// }

// impl Clone for SenderHandle {
//     fn clone(&self) -> Self {
//         Self {
//             inner: self.inner.clone(),
//             time_offset: self.time_offset,
//             path: self.path.clone(),
//             globals: self.globals.clone(),
//         }
//     }
// }
