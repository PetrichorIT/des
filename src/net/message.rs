use std::{
    collections::{LinkedList, VecDeque},
    fmt::Debug,
};

use super::*;
use crate::{Runtime, SimTime};

/// A runtime-unqiue identifier for a message or a message inherintance tree.
pub type MessageId = u32;
/// The type of messages, similar to the TOS field in IP packets.
pub type MessageKind = u16;

static mut MSG_COUNT: usize = 0;
fn register_message() {
    // unsafe { MSG_COUNT += 1 }
}

fn unregister_message() {
    // unsafe { MSG_COUNT -= 1 }
}

static mut NEXT_MESSAGE_ID: MessageId = 1;
fn get_message_id() -> MessageId {
    // SAFTEY:
    // Since we'll run single_threaded this be safe
    unsafe {
        let id = NEXT_MESSAGE_ID;
        NEXT_MESSAGE_ID += 1;
        id
    }
}

///
/// A generic network message holding a payload.
///
pub struct Message<T: MessageBody> {
    pub kind: MessageKind,
    pub timestamp: SimTime,
    pub content: T,

    // === Sender ===
    sender_module_id: ModuleId,

    // === Receiver ===
    target_module_id: ModuleId,

    // === Last Gate ===
    last_gate: GateId,
    hop_counter: usize,

    // === Timings ===
    creation_time: SimTime,
    send_time: SimTime,

    // === IDs ===
    message_id: MessageId,
    message_tree_id: MessageId,
}

impl<T: MessageBody> Message<T> {
    ///
    /// # Primitiv Getters
    ///

    #[inline(always)]
    pub fn sender_module_id(&self) -> ModuleId {
        self.sender_module_id
    }

    #[inline(always)]
    pub fn arrival_gate(&self) -> GateId {
        self.last_gate
    }

    #[inline(always)]
    pub fn target_module_id(&self) -> ModuleId {
        self.target_module_id
    }

    #[inline(always)]
    pub fn creation_time(&self) -> SimTime {
        self.creation_time
    }

    #[inline(always)]
    pub fn send_time(&self) -> SimTime {
        self.send_time
    }

    #[inline(always)]
    pub fn id(&self) -> MessageId {
        self.message_id
    }

    #[inline(always)]
    pub fn root_id(&self) -> MessageId {
        self.message_tree_id
    }

    ///
    /// # Additional fn
    ///

    #[inline(always)]
    pub fn is_self_msg(&self) -> bool {
        self.sender_module_id == self.target_module_id
    }

    #[inline(always)]
    pub fn register_hop(&mut self, gate_id: GateId) {
        self.last_gate = gate_id;
        self.hop_counter += 1;
    }

    pub fn set_arrival(&mut self, module_id: ModuleId, gate_id: GateId) {
        self.target_module_id = module_id;
        self.last_gate = gate_id;
    }

    ///
    /// # Constructors
    ///

    #[allow(clippy::too_many_arguments)]
    fn new_raw(
        kind: MessageKind,
        last_gate: GateId,
        hop_counter: usize,
        sender_module_id: ModuleId,
        target_module_id: ModuleId,
        creation_time: SimTime,
        send_time: SimTime,
        timestamp: SimTime,
        message_id: MessageId,
        message_tree_id: MessageId,
        content: T,
    ) -> Self {
        register_message();

        Self {
            kind,
            last_gate,
            hop_counter,
            sender_module_id,
            target_module_id,
            creation_time,
            send_time,
            timestamp,
            message_id,
            message_tree_id,
            content,
        }
    }

    pub fn new<A>(
        rt: &mut Runtime<A>,
        kind: MessageKind,
        last_gate: GateId,
        sender_module_id: ModuleId,
        target_module_id: ModuleId,
        timestamp: SimTime,
        content: T,
    ) -> Self {
        let id = get_message_id();
        Self::new_raw(
            kind,
            last_gate,
            0,
            sender_module_id,
            target_module_id,
            rt.sim_time(),
            SimTime::ZERO,
            timestamp,
            id,
            id,
            content,
        )
    }

    ///
    /// # Static methods
    ///

    pub fn total_message_count() -> usize {
        unsafe { MSG_COUNT }
    }
}

impl<T: MessageBody + Clone> Clone for Message<T> {
    fn clone(&self) -> Self {
        Self::new_raw(
            self.kind,
            self.last_gate,
            self.hop_counter,
            self.sender_module_id,
            self.target_module_id,
            self.creation_time,
            self.send_time,
            self.timestamp,
            get_message_id(),
            self.message_tree_id,
            self.content.clone(),
        )
    }
}

impl<T: MessageBody> Drop for Message<T> {
    fn drop(&mut self) {
        unregister_message()
    }
}

impl<T: MessageBody> Debug for Message<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("id", &self.message_id)
            .field("tree_id", &self.message_tree_id)
            .field("kind", &self.kind)
            .field("last_gate", &self.last_gate)
            .field("hop_counter", &self.hop_counter)
            .field("sender_module_id", &self.sender_module_id)
            .field("target_module_id", &self.target_module_id)
            .field(
                "timestamp",
                &format!(
                    "{} (created: {}, send: {})",
                    self.timestamp, self.creation_time, self.send_time
                ),
            )
            .finish()
    }
}

///
/// A trait that allows a type to be mesured in bits / bytes.
///
pub trait MessageBody {
    fn byte_len(&self) -> usize;
    fn bit_len(&self) -> usize {
        self.byte_len() * 8
    }
}

macro_rules! msg_body_primitiv {
    ($t: ty) => {
        impl MessageBody for $t {
            fn byte_len(&self) -> usize {
                std::mem::size_of::<Self>()
            }
        }
    };
}

msg_body_primitiv!(u8);
msg_body_primitiv!(u16);
msg_body_primitiv!(u32);
msg_body_primitiv!(u64);
msg_body_primitiv!(u128);
msg_body_primitiv!(usize);

msg_body_primitiv!(i8);
msg_body_primitiv!(i16);
msg_body_primitiv!(i32);
msg_body_primitiv!(i64);
msg_body_primitiv!(i128);
msg_body_primitiv!(isize);

msg_body_primitiv!(f64);
msg_body_primitiv!(f32);

msg_body_primitiv!(bool);
msg_body_primitiv!(char);

macro_rules! msg_body_lenable {
    ($t: ty) => {
        impl MessageBody for $t {
            fn byte_len(&self) -> usize {
                self.len()
            }
        }
    };
}

msg_body_lenable!(String);

// std::collections

impl<T: MessageBody> MessageBody for Vec<T> {
    fn byte_len(&self) -> usize {
        self.iter().fold(0, |acc, v| acc + v.byte_len())
    }
}

impl<T: MessageBody> MessageBody for VecDeque<T> {
    fn byte_len(&self) -> usize {
        self.iter().fold(0, |acc, v| acc + v.byte_len())
    }
}

impl<T: MessageBody> MessageBody for LinkedList<T> {
    fn byte_len(&self) -> usize {
        self.iter().fold(0, |acc, v| acc + v.byte_len())
    }
}

// [T]

impl<T: MessageBody> MessageBody for [T] {
    fn byte_len(&self) -> usize {
        self.iter().fold(0, |acc, v| acc + v.byte_len())
    }
}
