use std::{any::Any, fmt::Debug};

use crate::net::{GateRef, ModuleId, Packet};
use crate::time::SimTime;
use crate::util::Ptr;

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
    fn dup(&self) -> Self {
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

    pub(crate) content: Option<Box<dyn Any>>,
    pub(crate) bit_len: usize,
    pub(crate) byte_len: usize,
}

impl Message {
    ///
    /// The metadata attached to the message.
    ///
    #[inline]
    #[must_use]
    pub fn meta(&self) -> &MessageMetadata {
        &self.meta
    }

    ///
    /// A strinification function that reduces it to its identifering pars.
    ///
    #[must_use]
    pub fn str(&self) -> String {
        format!("#{} {} bits", self.meta.id, self.bit_len)
    }

    ///
    /// Creates a new instance of self through a builder.
    ///
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub fn new() -> MessageBuilder {
        MessageBuilder::new()
    }

    ///
    /// Consumes the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    ///
    #[must_use]
    pub fn try_cast<T: 'static + MessageBody + Send>(self) -> Option<(T, MessageMetadata)> {
        // SAFTY:
        // Since T is 'Send' this is safe within the bounds of Messages safty contract
        unsafe { self.try_cast_unsafe::<T>() }
    }

    ///
    /// Performs a [`try_cast`] unwraping the result.
    ///
    #[must_use]
    pub fn cast<T: 'static + MessageBody + Send>(self) -> (T, MessageMetadata) {
        self.try_cast().expect("Could not cast to type T")
    }

    ///
    /// Consumes the message casting the stored ptr
    /// into a Box of type T.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that the stored data is a valid instance
    /// of type T. If this cannot be guarnteed this is UB.
    /// Note that DES guarntees that the data refernced by ptr will not
    /// be freed until this function is called, and ownership is thereby moved..
    /// Note that this function allows T to be !Send. Be aware of safty problems arriving
    /// from this.
    ///
    #[must_use]
    pub unsafe fn try_cast_unsafe<T: 'static + MessageBody>(self) -> Option<(T, MessageMetadata)> {
        let Message { meta, content, .. } = self;
        let content = content?;
        let content = match content.downcast::<T>() {
            Ok(c) => c,
            Err(_) => return None,
        };

        let content = Box::into_inner(content);

        Some((content, meta))
    }

    ///
    /// Performs a [`try_cast_unsafe`] unwraping the result.
    ///
    /// # Safety
    ///
    /// See [`try_cast_unsafe`]
    #[must_use]
    pub unsafe fn cast_unsafe<T: 'static + MessageBody>(self) -> (T, MessageMetadata) {
        self.try_cast_unsafe().expect("Could not cast to type T")
    }

    ///
    /// A special cast for casting values that are packets.
    ///
    /// # Errors
    ///
    /// Returns itself if the convertion to a packet fails.
    ///
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub fn try_as_packet(self) -> Result<Packet, Self> {
        let Message {
            meta,
            content,
            bit_len,
            byte_len,
        } = self;
        // SAFTY:
        // This packet may contain a value that is used elsewhere,
        // but the metadate is used exclusivly.
        let pkt = content.map(|v| v.downcast::<Packet>());
        // let pkt = content.as_ref()?.downcast_ref::<Packet>().unwrap();

        let pkt = if let Some(pkt) = pkt {
            pkt
        } else {
            return Err(Message {
                meta,
                content: None,
                bit_len,
                byte_len,
            });
        };

        let pkt = match pkt {
            Ok(pkt) => pkt,
            Err(any) => {
                return Err(Message {
                    content: Some(any),
                    meta,
                    bit_len,
                    byte_len,
                })
            }
        };

        // This packet holds a reference to the same packet content but to
        // use message metadata & packet metadata ecxlusivly, new packets is created.
        let mut pkt: Packet = Box::into_inner(pkt);
        pkt.message_meta = Some(meta);

        Ok(pkt)
    }

    ///
    /// Casts a message into a packet preserving the messages metadata.
    ///
    #[must_use]
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
    #[inline]
    #[must_use]
    pub fn can_cast<T: 'static + MessageBody>(&self) -> bool {
        self.content.as_ref().map_or(false, |v| v.is::<T>())
    }
}

impl Message {
    ///
    /// Duplicates a message.
    ///
    /// # Panics
    ///
    /// Panics if the contained value is not of type T.
    ///
    #[must_use]
    pub fn dup<T>(&self) -> Self
    where
        T: 'static + Clone,
    {
        self.try_dup::<T>().expect("Failed to duplicate a message")
    }

    ///
    /// Tries to create a duplicate of the message, assuming its content is of type T.
    ///
    /// - If the messages body is of type T, the body will be cloned as specified by T
    /// and the dup will succeed.
    /// - If the message body is not of type T, this function will return `None`.
    /// - If the message has no body it will succeed independent of T and clone only the
    /// attached metadata.
    ///
    #[must_use]
    pub fn try_dup<T>(&self) -> Option<Self>
    where
        T: 'static + Clone,
    {
        let content: Option<Box<dyn Any>> = match &self.content {
            None => None,
            Some(boxed) => {
                let rf = boxed.downcast_ref::<T>()?;
                Some(Box::new(rf.clone()))
            }
        };

        let meta = self.meta.dup();
        let bit_len = self.bit_len;
        let byte_len = self.byte_len;

        Some(Self {
            meta,
            content,
            bit_len,
            byte_len,
        })
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

// SAFTY:
// A message only contains primitve data, ptrs that are threadsafe
// and a untyped contained value.
unsafe impl Send for Message {}

///
/// A trait that allows a type to be mesured in bits / bytes.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait MessageBody {
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

// # Primitives

macro_rules! msg_body_primitiv {
    ($t: ty) => {
        impl MessageBody for $t {
            fn byte_len(&self) -> usize {
                std::mem::size_of::<Self>()
            }
        }
    };
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

// # Basic types

impl<T: MessageBody> MessageBody for Box<T> {
    fn byte_len(&self) -> usize {
        use std::ops::Deref;

        self.deref().byte_len()
    }
}

impl<T: MessageBody> MessageBody for Option<T> {
    fn byte_len(&self) -> usize {
        match self {
            Some(ref content) => content.byte_len(),
            None => 0,
        }
    }
}

impl<T: MessageBody, E: MessageBody> MessageBody for Result<T, E> {
    fn byte_len(&self) -> usize {
        match self {
            Ok(ref val) => val.byte_len(),
            Err(ref err) => err.byte_len(),
        }
    }
}

// # Cells
use std::cell::{Cell, RefCell, UnsafeCell};

impl<T: MessageBody> MessageBody for Cell<T> {
    fn byte_len(&self) -> usize {
        // SAFTY: Since this is only used in this place, read only
        // this can be considered safe
        let val = unsafe { &*self.as_ptr() };
        val.bit_len()
    }
}

impl<T: MessageBody> MessageBody for RefCell<T> {
    fn byte_len(&self) -> usize {
        self.borrow().byte_len()
    }
}

impl<T: MessageBody> MessageBody for UnsafeCell<T> {
    fn byte_len(&self) -> usize {
        // SAFTY: Only used locally, read-only
        let val = unsafe { &*self.get() };
        val.byte_len()
    }
}

// # Collections
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};

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

impl<T: MessageBody, const N: usize> MessageBody for [T; N] {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for element in self {
            sum += element.byte_len();
        }
        sum
    }
}

impl<T: MessageBody> MessageBody for &[T] {
    fn byte_len(&self) -> usize {
        self.iter().fold(0, |acc, v| acc + v.byte_len())
    }
}

impl<K: MessageBody, V: MessageBody> MessageBody for HashMap<K, V> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for (k, v) in self.iter() {
            sum += k.byte_len() + v.byte_len()
        }
        sum
    }
}

impl<K: MessageBody, V: MessageBody> MessageBody for BTreeMap<K, V> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for (k, v) in self.iter() {
            sum += k.byte_len() + v.byte_len()
        }
        sum
    }
}

impl<T: MessageBody> MessageBody for HashSet<T> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for v in self.iter() {
            sum += v.byte_len()
        }
        sum
    }
}

impl<T: MessageBody> MessageBody for BTreeSet<T> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for v in self.iter() {
            sum += v.byte_len()
        }
        sum
    }
}

impl<T: MessageBody> MessageBody for BinaryHeap<T> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for v in self.iter() {
            sum += v.byte_len()
        }
        sum
    }
}

// # Tuples

impl<A, B> MessageBody for (A, B)
where
    A: MessageBody,
    B: MessageBody,
{
    fn byte_len(&self) -> usize {
        self.0.byte_len() + self.1.byte_len()
    }
}

impl<A, B, C> MessageBody for (A, B, C)
where
    A: MessageBody,
    B: MessageBody,
    C: MessageBody,
{
    fn byte_len(&self) -> usize {
        self.0.byte_len() + self.1.byte_len() + self.2.byte_len()
    }
}

impl<A, B, C, D> MessageBody for (A, B, C, D)
where
    A: MessageBody,
    B: MessageBody,
    C: MessageBody,
    D: MessageBody,
{
    fn byte_len(&self) -> usize {
        self.0.byte_len() + self.1.byte_len() + self.2.byte_len() + self.3.byte_len()
    }
}

impl<A, B, C, D, E> MessageBody for (A, B, C, D, E)
where
    A: MessageBody,
    B: MessageBody,
    C: MessageBody,
    D: MessageBody,
    E: MessageBody,
{
    fn byte_len(&self) -> usize {
        self.0.byte_len()
            + self.1.byte_len()
            + self.2.byte_len()
            + self.3.byte_len()
            + self.4.byte_len()
    }
}

impl<A, B, C, D, E, F> MessageBody for (A, B, C, D, E, F)
where
    A: MessageBody,
    B: MessageBody,
    C: MessageBody,
    D: MessageBody,
    E: MessageBody,
    F: MessageBody,
{
    fn byte_len(&self) -> usize {
        self.0.byte_len()
            + self.1.byte_len()
            + self.2.byte_len()
            + self.3.byte_len()
            + self.4.byte_len()
            + self.5.byte_len()
    }
}

// # Custom

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
    #[must_use]
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
    /// Returns the body, consuming `self`.
    ///
    #[must_use]
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

///
/// A intermediary type for constructing messages.
///
pub struct MessageBuilder {
    pub(crate) meta: MessageMetadata,
    pub(crate) content: Option<(usize, usize, Box<dyn Any>)>,
}

impl MessageBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            meta: MessageMetadata::default(),
            content: None,
        }
    }

    /// Sets the field `meta`.
    #[must_use]
    pub fn meta(mut self, meta: MessageMetadata) -> Self {
        self.meta = meta;
        self
    }

    /// Sets the field `meta.id`.
    #[must_use]
    pub fn id(mut self, id: MessageId) -> Self {
        self.meta.id = id;
        self
    }

    /// Sets the field `meta.kind`.
    #[must_use]
    pub fn kind(mut self, kind: MessageKind) -> Self {
        self.meta.kind = kind;
        self
    }

    /// Sets the field `meta.timestamp`.
    #[must_use]
    pub fn timestamp(mut self, timestamp: SimTime) -> Self {
        self.meta.timestamp = timestamp;
        self
    }

    /// Sets the field `meta.receiver_module_id`.
    #[must_use]
    pub fn receiver_module_id(mut self, receiver_module_id: ModuleId) -> Self {
        self.meta.receiver_module_id = receiver_module_id;
        self
    }

    /// Sets the field `meta.sender_module_id`.
    #[must_use]
    pub fn sender_module_id(mut self, sender_module_id: ModuleId) -> Self {
        self.meta.sender_module_id = sender_module_id;
        self
    }

    /// Sets the field `meta.last_gate`.
    #[must_use]
    pub fn last_gate(mut self, last_gate: GateRef) -> Self {
        self.meta.last_gate = Some(last_gate);
        self
    }

    /// Sets the field `meta`.`creation_time`.
    #[must_use]
    pub fn creation_time(mut self, creation_time: SimTime) -> Self {
        self.meta.creation_time = creation_time;
        self
    }

    /// Sets the field `meta.send_time`.
    #[must_use]
    pub fn send_time(mut self, send_time: SimTime) -> Self {
        self.meta.send_time = send_time;
        self
    }

    /// Sets the field `content`.
    #[must_use]
    pub fn content<T>(mut self, content: T) -> Self
    where
        T: 'static + MessageBody + Send,
    {
        let bit_len = content.bit_len();
        let byte_len = content.byte_len();
        let interned = Box::new(content);
        self.content = Some((bit_len, byte_len, interned));
        self
    }

    /// Sets the field `content`.
    #[must_use]
    pub fn content_boxed<T>(mut self, content: Box<T>) -> Self
    where
        T: 'static + MessageBody + Send,
    {
        self.content = Some((content.bit_len(), content.byte_len(), content));
        self
    }

    /// Builds a message from the builder.
    #[must_use]
    pub fn build(self) -> Message {
        let MessageBuilder { meta, content } = self;

        let (bit_len, byte_len, content) = match content {
            Some((bit_len, byte_len, content)) => (bit_len, byte_len, Some(content)),
            None => (0, 0, None),
        };

        Message {
            meta,
            content,
            bit_len,
            byte_len,
        }
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for MessageBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MessageBuilder")
    }
}

// SAFTY:
// Dervived from safty invariants of [Message].
unsafe impl Send for MessageBuilder {}
