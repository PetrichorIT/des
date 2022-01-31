use std::{
    collections::{LinkedList, VecDeque},
    fmt::Debug,
};

use crate::core::interning::*;
use crate::core::*;
use crate::create_global_uid;
use crate::net::*;

create_global_uid!(
    /// A globaly unqiue identifer for a message.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
    pub MessageId(u32) = MESSAGE_ID;
);

/// The type of messages, similar to the TOS field in IP packets.
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub type MessageKind = u16;

///
/// The metadata attachted to a message, independent of its contents.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageMetadata {
    /// A unqiue identifier for this instance of a message.
    pub id: MessageId,
    /// A unique identifier for the message this instance was cloned from.
    /// If this instance was not cloned 'tree_id == id'.
    pub tree_id: MessageId,

    /// The type of message to be handled.
    pub kind: MessageKind,
    /// A custom user-defined timestamp.
    pub timestamp: SimTime,

    /// The id of the module that send this message.
    pub sender_module_id: ModuleId,
    /// The id of the module that received this message.
    /// This is 'MODULE_NULL' until the message is received at a module.
    pub receiver_module_id: ModuleId,

    /// The last gate the message was passed through.
    /// This can be used to identifier the inbound port
    /// of a module.
    pub last_gate: GateId,

    /// A timestamp when the message was created.
    pub creation_time: SimTime,
    /// A timestamp when the message was send onto a link.
    /// This may differ from the creation time if either messages are relayed
    /// with processing delay, or some kind of buffered queue delays the transmission
    /// onto the link.
    pub send_time: SimTime,
}

impl MessageMetadata {
    fn clone_message(&self) -> Self {
        Self {
            id: MessageId::gen(),
            tree_id: self.id,

            kind: self.kind,
            timestamp: self.timestamp,

            sender_module_id: self.sender_module_id,
            receiver_module_id: self.receiver_module_id,

            last_gate: self.last_gate,

            creation_time: SimTime::now(),
            send_time: SimTime::MAX,
        }
    }
}

///
/// A generic network message holding a payload.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub struct Message {
    meta: MessageMetadata,

    content: InternedValue<'static>,
    bit_len: usize,
    byte_len: usize,
}

impl Message {
    ///
    /// # Primitiv Getters
    ///

    pub fn kind(&self) -> MessageKind {
        self.meta.kind
    }

    #[inline(always)]
    pub fn sender_module_id(&self) -> ModuleId {
        self.meta.sender_module_id
    }

    #[inline(always)]
    pub fn arrival_gate(&self) -> GateId {
        self.meta.last_gate
    }

    #[inline(always)]
    pub fn receiver_module_id(&self) -> ModuleId {
        self.meta.receiver_module_id
    }

    #[inline(always)]
    pub fn creation_time(&self) -> SimTime {
        self.meta.creation_time
    }

    #[inline(always)]
    pub fn send_time(&self) -> SimTime {
        self.meta.send_time
    }

    #[inline(always)]
    pub fn id(&self) -> MessageId {
        self.meta.id
    }

    #[inline(always)]
    pub fn root_id(&self) -> MessageId {
        self.meta.tree_id
    }

    pub fn bit_len(&self) -> usize {
        self.bit_len
    }

    pub fn set_last_gate(&mut self, gate: GateId) {
        self.meta.last_gate = gate;
    }

    ///
    /// # Additional fn
    ///

    #[inline(always)]
    pub fn is_self_msg(&self) -> bool {
        self.meta.sender_module_id == self.meta.receiver_module_id
    }

    pub fn set_target_module(&mut self, module_id: ModuleId) {
        self.meta.receiver_module_id = module_id;
    }

    pub fn str(&self) -> String {
        format!(
            "#{}({}) {} bits",
            self.meta.id, self.meta.tree_id, self.bit_len
        )
    }

    ///
    /// # Constructors
    ///

    #[allow(clippy::too_many_arguments)]
    fn new_raw(
        meta: MessageMetadata,
        content: InternedValue<'static>,
        bit_len: usize,
        byte_len: usize,
    ) -> Self {
        Self {
            meta,
            content,
            bit_len,
            byte_len,
        }
    }

    ///
    /// Creates a new message with the given metadata and
    /// a content of type Box<T>.
    ///
    /// # Guarntees
    ///
    /// The value of type T will be moved into a box which is then
    /// transmuted into a raw ptr. The allocated memory of T will only
    /// be dropped if the message is extracted.
    ///
    pub fn new_interned<T: MessageBody>(
        kind: MessageKind,
        sender_module_id: ModuleId,
        timestamp: SimTime,
        content: TypedInternedValue<'static, T>,
    ) -> Self {
        let bit_len = content.bit_len();
        let byte_len = content.byte_len();

        let id = MessageId::gen();

        let meta = MessageMetadata {
            id,
            tree_id: id,

            kind,
            timestamp,

            sender_module_id,
            receiver_module_id: MODULE_NULL,

            last_gate: GATE_NULL,

            creation_time: SimTime::now(),
            send_time: SimTime::MAX,
        };

        Self::new_raw(meta, content.uncast(), bit_len, byte_len)
    }

    ///
    /// Creates a new message with the given metadata and
    /// a content of type T.
    ///
    /// # Guarntees
    ///
    /// The value of type T will be moved into a box which is then
    /// transmuted into a raw ptr. The allocated memory of T will only
    /// be dropped if the message is extracted.
    ///
    pub fn new<T: 'static + MessageBody>(
        kind: MessageKind,
        last_gate: GateId,
        sender_module_id: ModuleId,
        receiver_module_id: ModuleId,
        timestamp: SimTime,
        content: T,
    ) -> Self {
        let id = MessageId::gen();

        let bit_len = content.bit_len();
        let byte_len = content.byte_len();

        let interned = unsafe { (*RTC.get()).as_ref().unwrap().interner.intern(content) };

        let meta = MessageMetadata {
            id,
            tree_id: id,

            kind,
            timestamp,

            sender_module_id,
            receiver_module_id,

            last_gate,

            creation_time: SimTime::now(),
            send_time: SimTime::MAX,
        };

        Self::new_raw(meta, interned, bit_len, byte_len)
    }

    ///
    /// Consumes the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// # Safty
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    ///
    pub fn cast<T: MessageBody>(self) -> (TypedInternedValue<'static, T>, MessageMetadata) {
        let Message { meta, content, .. } = self;
        (content.cast(), meta)
    }

    ///
    /// Indicates wheter a cast to a instance of type T ca
    /// succeed.
    ///
    /// # Safty
    ///
    /// Note that this only gurantees that a cast will result in UB
    /// if it returns 'false'. Should this function return 'true' it indicates
    /// that the underlying value was created as a instance of type 'T',
    /// which does not gurantee that this is a internally valid instance
    /// of 'T'.
    ///
    pub fn can_cast<T: 'static + MessageBody>(&self) -> bool {
        self.content.can_cast::<T>()
    }
}

impl Clone for Message {
    fn clone(&self) -> Self {
        Self::new_raw(
            self.meta.clone_message(),
            self.content.clone(),
            self.bit_len,
            self.byte_len,
        )
    }
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("id", &self.meta.id)
            .field("tree_id", &self.meta.tree_id)
            .field("kind", &self.meta.kind)
            .field("last_gate", &self.meta.last_gate)
            .field("sender_module_id", &self.meta.sender_module_id)
            .field("target_module_id", &self.meta.receiver_module_id)
            .field(
                "timestamp",
                &format!(
                    "{} (created: {}, send: {})",
                    self.meta.timestamp, self.meta.creation_time, self.meta.send_time
                ),
            )
            .finish()
    }
}

///
/// A trait that allows a type to be mesured in bits / bytes.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
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
