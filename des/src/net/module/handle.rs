use crate::net::*;
use crate::time::*;

pub(crate) enum BufferEvent {
    Send {
        msg: Message,
        time_offset: Duration,
        out: Box<dyn IntoModuleGate>,
    },
    ScheduleIn {
        msg: Message,
        time_offset: Duration,
    },
    ScheduleAt {
        msg: Message,
        time: SimTime,
    },
}

impl std::fmt::Debug for BufferEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Send {
                msg, time_offset, ..
            } => f
                .debug_struct("BufferEvent::Send")
                .field("msg", msg)
                .field("time_offset", time_offset)
                .finish(),
            Self::ScheduleIn { msg, time_offset } => f
                .debug_struct("BufferEvent::ScheduleIn")
                .field("msg", msg)
                .field("time_offset", time_offset)
                .finish(),
            Self::ScheduleAt { msg, time } => f
                .debug_struct("BufferEvent::ScheduleAt")
                .field("msg", msg)
                .field("time", time)
                .finish(),
        }
    }
}

unsafe impl Send for BufferEvent {}

///
/// The asychronous part of a send handle.
/// This can be Send via threads to enqueu messages
/// from a module.
///
pub struct HandleSender {
    pub(super) inner: tokio::sync::mpsc::UnboundedSender<BufferEvent>,
    pub(super) time_offset: Duration,
}

impl HandleSender {
    ///
    /// Adds the duration to the processing time offset.
    /// All messages send after this time will be delayed by the
    /// processing time delay.
    ///
    pub fn processing_time(&mut self, duration: Duration) {
        self.time_offset += duration;
    }

    ///
    /// Sends a message onto a given gate. This operation will be performed after
    /// handle_message finished.
    ///
    pub fn send(&self, msg: impl Into<Message>, gate: impl IntoModuleGate + 'static) {
        self.inner
            .send(BufferEvent::Send {
                msg: msg.into(),
                time_offset: self.time_offset,
                out: Box::new(gate),
            })
            .expect("Failed to send to unbounded channel. reciver must have died.");
    }

    ///
    /// Enqueues a event that will trigger the [Module::handle_message] function
    /// in duration seconds, shifted by the processing time delay.
    ///
    pub fn schedule_in(&self, msg: impl Into<Message>, duration: Duration) {
        self.inner
            .send(BufferEvent::ScheduleIn {
                msg: msg.into(),
                time_offset: self.time_offset + duration,
            })
            .expect("Failed to send to unbounded channel. reciver must have died.");
    }

    ///
    /// Enqueues a event that will trigger the [Module::handle_message] function
    /// at the given SimTime
    ///
    pub fn schedule_at(&self, msg: impl Into<Message>, time: SimTime) {
        self.inner
            .send(BufferEvent::ScheduleAt {
                msg: msg.into(),
                time: time,
            })
            .expect("Failed to send to unbounded channel. reciver must have died.");
    }
}

impl Clone for HandleSender {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            time_offset: self.time_offset,
        }
    }
}
