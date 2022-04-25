use std::{
    any::Any,
    collections::{LinkedList, VecDeque},
    fmt::Debug,
    rc::Rc,
};

use crate::core::*;
use crate::net::*;
use crate::util::*;

use log::warn;

///
/// A ID that defines the meaning of the message in the simulation context.
///
///  * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub type MessageId = u16;

///
/// The type of messages, similar to the TOS field in IP packets.
///
///  * This type is only available of DES is build with the `"net"` feature.*
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
    pub last_gate: Option<GateRef>,

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
            id: self.id,

            kind: self.kind,
            timestamp: self.timestamp,

            sender_module_id: self.sender_module_id,
            receiver_module_id: self.receiver_module_id,

            last_gate: self.last_gate.as_ref().map(Ptr::clone),

            creation_time: SimTime::now(),
            send_time: SimTime::MAX,
        }
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            id: 0,
            kind: 0,
            timestamp: SimTime::MIN,
            sender_module_id: ModuleId::NULL,
            receiver_module_id: ModuleId::NULL,
            last_gate: None,
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
    pub(crate) meta: MessageMetadata,

    pub(crate) content: Option<Rc<dyn Any>>,
    pub(crate) bit_len: usize,
    pub(crate) byte_len: usize,
}

impl Message {
    ///
    /// The metadata attached to the message.
    ///
    #[inline(always)]
    pub fn meta(&self) -> &MessageMetadata {
        &self.meta
    }

    ///
    /// A strinification function that reduces it to its identifering pars.
    ///
    pub fn str(&self) -> String {
        format!("#{} {} bits", self.meta.id, self.bit_len)
    }

    ///
    /// Creates a new instance of self through a builder.
    ///
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> MessageBuilder {
        MessageBuilder::new()
    }

    ///
    /// Consumes the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// ## Safty
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    ///

    pub fn try_cast<T: 'static + MessageBody>(self) -> Option<(T, MessageMetadata)> {
        let Message { meta, content, .. } = self;
        let content = match content?.downcast::<T>() {
            Ok(c) => c,
            Err(_) => return None,
        };

        let content = match Rc::try_unwrap(content) {
            Ok(c) => c,
            //
            Err(c) => {
                warn!(target: "des", "Multiple messages refered to the same content: Cloned content at `try_cast`.");
                (*c).clone()
            }
        };

        Some((content, meta))
    }

    pub fn cast<T: 'static + MessageBody>(self) -> (T, MessageMetadata) {
        self.try_cast().expect("Could not cast to type T")
    }

    ///
    /// A special cast for casting values that are packets.
    ///
    ///

    pub fn try_as_packet(self) -> Option<Packet> {
        let Message { meta, content, .. } = self;
        // SAFTY:
        // This packet may contain a value that is used elsewhere,
        // but the metadate is used exclusivly.
        let pkt = content.as_ref()?.downcast_ref::<Packet>().unwrap();

        // This packet holds a reference to the same packet content but to
        // use message metadata & packet metadata ecxlusivly, new packets is created.
        let mut pkt: Packet = pkt.clone();
        pkt.message_meta = Some(meta);

        Some(pkt)
    }

    pub fn as_packet(self) -> Packet {
        self.try_as_packet().expect("Could not cast self to packet")
    }

    ///
    /// Indicates wheter a cast to a instance of type T ca
    /// succeed.
    ///
    /// ## Safty
    ///
    /// Note that this only gurantees that a cast will result in UB
    /// if it returns 'false'. Should this function return 'true' it indicates
    /// that the underlying value was created as a instance of type 'T',
    /// which does not gurantee that this is a internally valid instance
    /// of 'T'.
    ///
    #[inline(always)]
    pub fn can_cast<T: 'static + MessageBody>(&self) -> bool {
        self.content.as_ref().map(|v| v.is::<T>()).unwrap_or(false)
    }
}

impl Clone for Message {
    fn clone(&self) -> Self {
        Self {
            meta: self.meta.clone_message(),
            content: self.content.clone(),
            bit_len: self.bit_len,
            byte_len: self.byte_len,
        }
    }
}

impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("id", &self.meta.id)
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
pub trait MessageBody: Clone {
    ///
    /// The length of the message body in bytes.
    ///
    fn byte_len(&self) -> usize;

    ///
    /// The length of the message body in bits.
    /// This should be the byte length time 8 generally
    /// but should be implemented otherwise for small datatypes.
    ///
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

///
/// A message body that does mimics a custom size
/// independet of actualy size.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CustomSizeBody<T> {
    bit_len: usize,
    inner: T,
}

impl<T> CustomSizeBody<T> {
    ///
    /// Creates a new instance of `Self`.
    ///
    pub fn new(bit_len: usize, inner: T) -> Self {
        Self { bit_len, inner }
    }

    ///
    /// Returns a reference to the real contained body.
    ///
    pub fn inner(&self) -> &T {
        &self.inner
    }

    ///
    /// Returns a mutable reference to the real contained body.
    ///
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    ///
    /// Returns the body, consuming `self``.
    ///
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> MessageBody for CustomSizeBody<T>
where
    T: Clone,
{
    fn byte_len(&self) -> usize {
        self.bit_len / 8
    }

    fn bit_len(&self) -> usize {
        self.bit_len
    }
}

msg_body_primitiv!(());

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

impl<T: MessageBody> MessageBody for &[T] {
    fn byte_len(&self) -> usize {
        self.iter().fold(0, |acc, v| acc + v.byte_len())
    }
}

///
/// A intermediary type for constructing messages.
///
pub struct MessageBuilder {
    pub(crate) meta: MessageMetadata,
    pub(crate) content: Option<(usize, usize, Rc<dyn Any>)>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self {
            meta: MessageMetadata::default(),
            content: None,
        }
    }

    pub fn meta(mut self, meta: MessageMetadata) -> Self {
        self.meta = meta;
        self
    }

    pub fn id(mut self, id: MessageId) -> Self {
        self.meta.id = id;
        self
    }

    pub fn kind(mut self, kind: MessageKind) -> Self {
        self.meta.kind = kind;
        self
    }

    pub fn timestamp(mut self, timestamp: SimTime) -> Self {
        self.meta.timestamp = timestamp;
        self
    }

    pub fn receiver_module_id(mut self, receiver_module_id: ModuleId) -> Self {
        self.meta.receiver_module_id = receiver_module_id;
        self
    }

    pub fn sender_module_id(mut self, sender_module_id: ModuleId) -> Self {
        self.meta.sender_module_id = sender_module_id;
        self
    }

    pub fn last_gate(mut self, last_gate: GateRef) -> Self {
        self.meta.last_gate = Some(last_gate);
        self
    }

    pub fn creation_time(mut self, creation_time: SimTime) -> Self {
        self.meta.creation_time = creation_time;
        self
    }

    pub fn send_time(mut self, send_time: SimTime) -> Self {
        self.meta.send_time = send_time;
        self
    }

    pub fn content<T>(mut self, content: T) -> Self
    where
        T: 'static + MessageBody,
    {
        let bit_len = content.bit_len();
        let byte_len = content.byte_len();
        let interned = Rc::new(content);
        self.content = Some((bit_len, byte_len, interned));
        self
    }

    pub fn content_boxed<T>(mut self, content: Rc<T>) -> Self
    where
        T: 'static + MessageBody,
    {
        self.content = Some((content.bit_len(), content.byte_len(), content));
        self
    }

    pub fn build(self) -> Message {
        let MessageBuilder { meta, content } = self;

        let (bit_len, byte_len, content) = match content {
            Some((bit_len, byte_len, content)) => (bit_len, byte_len, Some(content)),
            None => (0, 0, None),
        };

        Message {
            meta,
            bit_len,
            byte_len,
            content,
        }
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}
