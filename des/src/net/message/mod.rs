use crate::net::{GateRef, ModuleId};
use crate::time::SimTime;
use std::fmt::Debug;
use std::net::{IpAddr, SocketAddr};

mod body;
pub use body::{CustomSizeBody, MessageBody};

mod util;
use util::AnyBox;

mod header;
#[allow(unused_imports)]
pub(crate) use header::*;
pub use header::{MessageHeader, MessageId, MessageKind, MessageType};

///
/// A generic network message holding a payload.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
pub struct Message {
    pub(crate) header: MessageHeader,
    pub(crate) content: Option<AnyBox>,
}

impl Message {
    ///
    /// Creates a new instance of self through a builder.
    ///
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub fn new() -> MessageBuilder {
        MessageBuilder::new()
    }

    /// Returns the length of the complete message
    #[must_use]
    pub fn length(&self) -> usize {
        self.header.length as usize + self.header.byte_len()
    }

    ///
    /// The metadata attached to the message.
    ///
    #[inline]
    #[must_use]
    pub fn header(&self) -> &MessageHeader {
        &self.header
    }

    ///
    /// A strinification function that reduces it to its identifering pars.
    ///
    #[must_use]
    pub fn str(&self) -> String {
        format!(
            "Message {{ {} bytes {} }}",
            self.header.length,
            self.content.as_ref().map_or("no content", AnyBox::ty)
        )
    }
}

/// # Special accessors
impl Message {
    ///
    /// Registers a hop in the header, thereby decrementing ttl
    /// while incrementing the hop count.
    ///
    pub fn register_hop(&mut self) {
        self.header.ttl = self.header.ttl.wrapping_sub(1);
        self.header.hop_count += 1;
    }
}

/// # Content Accessing
impl Message {
    ///
    /// Trys to return the content by reference casted to the given type T.
    /// Returns [None] if the no content exists or the content is not of type T.
    ///
    #[must_use]
    pub fn try_content<T: 'static + MessageBody>(&self) -> Option<&T> {
        Some(self.content.as_ref()?.try_cast_ref::<T>())?
    }

    ///
    /// Trys to return the content by reference casted to the given type T.
    /// Panics if the no content exists or the content is not of type T.
    ///
    #[must_use]
    pub fn content<T: 'static + MessageBody>(&self) -> &T {
        self.try_content().expect("Failed to unwrap")
    }

    ///
    /// Trys to return the content by mutable ref casted to the given type T.
    /// Returns [None] if the no content exists or the content is not of type T.
    ///
    pub fn try_content_mut<T: 'static + MessageBody>(&mut self) -> Option<&mut T> {
        Some(self.content.as_mut()?.try_cast_mut())?
    }

    ///
    /// Trys to return the content by mutable ref casted to the given type T.
    /// Panics if the no content exists or the content is not of type T.
    ///
    pub fn content_mut<T: 'static + MessageBody>(&mut self) -> &mut T {
        self.try_content_mut().expect("Failed to unwrap")
    }
}

/// # Content casting
impl Message {
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
        self.content.as_ref().map_or(false, AnyBox::can_cast::<T>)
    }

    ///
    /// Performs a [`try_cast`] unwraping the result.
    ///
    #[must_use]
    pub fn cast<T: 'static + MessageBody + Send>(self) -> (T, MessageHeader) {
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
    ///
    /// # Errors
    ///
    /// Returns an error if either there is no content, or
    /// the content is not of type T.
    pub fn try_cast<T: 'static + MessageBody + Send>(self) -> Result<(T, MessageHeader), Self> {
        // SAFTY:
        // Since T is 'Send' this is safe within the bounds of Messages safty contract
        unsafe { self.try_cast_unsafe::<T>() }
    }

    ///
    /// Performs a [`try_cast_unsafe`] unwraping the result.
    ///
    /// # Safety
    ///
    /// See [`try_cast_unsafe`]
    #[must_use]
    pub unsafe fn cast_unsafe<T: 'static + MessageBody>(self) -> (T, MessageHeader) {
        self.try_cast_unsafe().expect("Could not cast to type T")
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
    /// # Errors
    ///
    /// Returns an error if either there is no content,
    /// or the content is not of type T.
    ///

    pub unsafe fn try_cast_unsafe<T: 'static + MessageBody>(
        self,
    ) -> Result<(T, MessageHeader), Self> {
        let Message { header, content } = self;
        let content = match content.map(|c| c.try_cast_unsafe::<T>()) {
            Some(Ok(c)) => c,
            Some(Err(content)) => {
                return Err(Self {
                    header,
                    content: Some(content),
                })
            }
            None => {
                return Err(Self {
                    header,
                    content: None,
                })
            }
        };

        Ok((content, header))
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
        let content: Option<AnyBox> = if let Some(ref content) = self.content {
            Some(content.try_dup::<T>()?)
        } else {
            None
        };

        let header = self.header.dup();

        Some(Self { header, content })
    }
}

// SAFTY:
// A message only contains primitve data, ptrs that are threadsafe
// and a untyped contained value.
unsafe impl Send for Message {}

///
/// A intermediary type for constructing messages.
///
pub struct MessageBuilder {
    pub(crate) header: MessageHeader,
    pub(crate) content: Option<AnyBox>,
}

impl MessageBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: MessageHeader::default(),
            content: None,
        }
    }

    /// Only internal use
    #[allow(unused)]
    pub(crate) fn typ(mut self, typ: u8) -> Self {
        self.header.typ = typ;
        self
    }

    /// Sets the field `meta`.
    #[must_use]
    pub fn header(mut self, meta: MessageHeader) -> Self {
        self.header = meta;
        self
    }

    /// Sets the field `header.id`.
    #[must_use]
    pub fn id(mut self, id: MessageId) -> Self {
        self.header.id = id;
        self
    }

    /// Sets the field `header.kind`.
    #[must_use]
    pub fn kind(mut self, kind: MessageKind) -> Self {
        self.header.kind = kind;
        self
    }

    /// Sets the field `header.receiver_module_id`.
    #[must_use]
    pub fn receiver_module_id(mut self, receiver_module_id: ModuleId) -> Self {
        self.header.receiver_module_id = receiver_module_id;
        self
    }

    /// Sets the field `header.sender_module_id`.
    #[must_use]
    pub fn sender_module_id(mut self, sender_module_id: ModuleId) -> Self {
        self.header.sender_module_id = sender_module_id;
        self
    }

    /// Sets the field `header.last_gate`.
    #[must_use]
    pub fn last_gate(mut self, last_gate: GateRef) -> Self {
        self.header.last_gate = Some(last_gate);
        self
    }

    /// Sets the field `meta`.`creation_time`.
    #[must_use]
    pub fn creation_time(mut self, creation_time: SimTime) -> Self {
        self.header.creation_time = creation_time;
        self
    }

    /// Sets the field `header.send_time`.
    #[must_use]
    pub fn send_time(mut self, send_time: SimTime) -> Self {
        self.header.send_time = send_time;
        self
    }

    /// Sets the field `src_node` and `src_port`.
    #[must_use]
    pub fn src(mut self, src_addr: SocketAddr) -> Self {
        self.header.src_addr = src_addr;
        self
    }

    /// Sets the field `src_node`.
    #[must_use]
    pub fn src_node(mut self, src_node: IpAddr) -> Self {
        self.header.src_addr.set_ip(src_node);
        self
    }

    /// Sets the field `src_port`.
    #[must_use]
    pub fn src_port(mut self, src_port: u16) -> Self {
        self.header.src_addr.set_port(src_port);
        self
    }

    /// Sets the field `dest_node` and `dest_port`
    #[must_use]
    pub fn dest(mut self, dest_addr: SocketAddr) -> Self {
        self.header.dest_addr = dest_addr;
        self
    }

    /// Sets the field `dest_node`.
    #[must_use]
    pub fn dest_node(mut self, dest_node: IpAddr) -> Self {
        self.header.dest_addr.set_ip(dest_node);
        self
    }

    /// Sets the field `dest_port`.
    #[must_use]
    pub fn dest_port(mut self, dest_port: u16) -> Self {
        self.header.dest_addr.set_port(dest_port);
        self
    }

    /// Sets the field `seq_no`.
    #[must_use]
    pub fn seq_no(mut self, seq_no: u32) -> Self {
        self.header.seq_no = seq_no;
        self
    }

    /// Sets the field `content`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn content<T>(mut self, content: T) -> Self
    where
        T: 'static + MessageBody + Send,
    {
        self.header.length = content.byte_len() as u32;
        self.content = Some(AnyBox::new(content));
        self
    }

    /// Sets the field `content`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn content_boxed<T>(mut self, content: Box<T>) -> Self
    where
        T: 'static + MessageBody + Send,
    {
        self.header.length = content.byte_len() as u32;
        self.content = Some(AnyBox::new(Box::into_inner(content)));
        self
    }

    /// Builds a message from the builder.
    #[must_use]
    pub fn build(self) -> Message {
        let MessageBuilder { header, content } = self;

        Message { header, content }
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