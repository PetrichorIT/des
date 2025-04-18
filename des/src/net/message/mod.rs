//! Generic network messages.

use crate::net::{gate::GateRef, module::ModuleId};
use crate::time::SimTime;
use std::any::Any;
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use std::panic::UnwindSafe;

mod api;
pub use api::*;

mod body;
pub use body::*;

mod header;
pub use header::*;

///
/// A network message holding a arbitrary payload.
///
/// A message is composed from two parts:
/// - a `Header` containing generic message parameters
/// - and a optional `Body`, containing an arbitrary payload.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug, Default)]
#[must_use]
pub struct Message {
    pub(crate) header: Box<Header>,
    pub(crate) content: Option<Body>,
}

impl Message {
    /// Constructs a message from its raw parts.
    ///
    /// The header is boxed for improved internal layout.
    pub fn from_raw_parts(header: Box<Header>, body: Option<Body>) -> Self {
        Self {
            header,
            content: body,
        }
    }

    /// From parts
    pub fn from_parts<T: MessageBody + Any + Clone + Debug>(
        header: Header,
        body: Option<T>,
    ) -> Self {
        Self::from_raw_parts(Box::new(header), body.map(Body::new))
    }

    /// Returns the length of the complete message.
    ///
    /// The length is the sum of the bodys length and a fixed header length.
    #[must_use]
    pub fn length(&self) -> usize {
        self.content.as_ref().map_or(0, Body::length) + self.header.byte_len()
    }

    /// The metadata attached to the message.
    #[inline]
    #[must_use]
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// The metadata attached to the message.
    #[inline]
    #[must_use]
    pub fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }
}

// # Header fields

impl Deref for Message {
    type Target = Header;
    fn deref(&self) -> &Self::Target {
        &self.header
    }
}

impl DerefMut for Message {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.header
    }
}

// # Header fields, builders

impl Message {
    /// **Builder** that sets the messsage ID field.
    pub fn id(mut self, id: MessageId) -> Self {
        self.header.id = id;
        self
    }

    /// **Builder** that sets the messsage kind field.
    pub fn kind(mut self, kind: MessageKind) -> Self {
        self.header.kind = kind;
        self
    }

    /// **Builder** that sets the creation time field.
    pub fn creation_time(mut self, time: SimTime) -> Self {
        self.header.creation_time = time;
        self
    }

    /// **Builder** that sets the send time field.
    pub fn send_time(mut self, time: SimTime) -> Self {
        self.header.send_time = time;
        self
    }

    /// **Builder** that sets the sender module ID field.
    pub fn sender_module_id(mut self, id: ModuleId) -> Self {
        self.header.sender_module_id = id;
        self
    }

    /// **Builder** that sets the receiver module ID field.
    pub fn receiver_module_id(mut self, id: ModuleId) -> Self {
        self.header.receiver_module_id = id;
        self
    }

    /// **Builder** that sets the last gate field.
    pub fn last_gate(mut self, gate: GateRef) -> Self {
        self.header.last_gate = Some(gate);
        self
    }

    /// **Builder** that sets the source MAC address field.
    pub fn src(mut self, src: [u8; 6]) -> Self {
        self.header.src = src;
        self
    }

    /// **Builder** that sets the destination MAC address field.
    pub fn dst(mut self, dest: [u8; 6]) -> Self {
        self.header.dst = dest;
        self
    }
}

// # Content Accessing

impl Message {
    /// Sets the content of the message.
    pub fn set_body(&mut self, body: Body) {
        self.content = Some(body);
    }

    /// Sets the content of the message.
    pub fn set_content<T>(&mut self, value: T)
    where
        T: MessageBody + Clone + Debug + Any,
    {
        self.content = Some(Body::new(value));
    }

    /// Sets the content of the message.
    pub fn set_content_non_clonable<T>(&mut self, value: T)
    where
        T: MessageBody + Debug + Any,
    {
        self.content = Some(Body::new_non_clonable(value));
    }

    /// Sets the content of the message.
    pub fn set_content_non_debugable<T>(&mut self, value: T)
    where
        T: Clone + Any,
    {
        self.content = Some(Body::new_non_debugable(value));
    }

    /// **Builder** that sets the content of the message.
    pub fn with_body(mut self, body: Body) -> Self {
        self.set_body(body);
        self
    }

    /// **Builder** that sets the content of the message.
    pub fn with_content<T>(mut self, body: T) -> Self
    where
        T: MessageBody + Clone + Debug + Any,
    {
        self.set_content(body);
        self
    }

    /// Trys to return the content by reference casted to the given type T.
    /// Returns [None] if the no content exists or the content is not of type T.
    #[must_use]
    pub fn try_content<T: 'static + MessageBody>(&self) -> Option<&T> {
        self.content.as_ref().and_then(Body::try_content::<T>)
    }

    /// Trys to return the content by reference casted to the given type T.
    /// Panics if the no content exists or the content is not of type T.
    ///
    /// # Panics
    ///
    /// Panics if he cast fails.
    #[must_use]
    pub fn content<T: 'static + MessageBody>(&self) -> &T {
        self.try_content().expect("Failed to unwrap")
    }

    /// Trys to return the content by mutable ref casted to the given type T.
    /// Returns [None] if the no content exists or the content is not of type T.
    pub fn try_content_mut<T: 'static + MessageBody>(&mut self) -> Option<&mut T> {
        self.content.as_mut().and_then(Body::try_content_mut::<T>)
    }

    /// Trys to return the content by mutable ref casted to the given type T.
    /// Panics if the no content exists or the content is not of type T.
    ///
    /// # Panics
    ///
    /// Panics if he cast fails.
    pub fn content_mut<T: 'static + MessageBody>(&mut self) -> &mut T {
        self.try_content_mut().expect("Failed to unwrap")
    }

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
    /// # Panics
    ///
    /// Panics if he cast fails.
    #[inline]
    #[must_use]
    pub fn can_cast<T: 'static + MessageBody>(&self) -> bool {
        self.content.as_ref().map_or(false, Body::is::<T>)
    }

    /// Performs a [`try_cast`](Message::try_cast)unwraping the result.
    ///
    /// # Panics
    ///
    /// Panics if he cast fails.
    #[must_use]
    pub fn cast<T: 'static + MessageBody + Send>(self) -> (T, Header) {
        self.try_cast().expect("Could not cast to type T")
    }

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
    pub fn try_cast<T: 'static + MessageBody + Send>(self) -> Result<(T, Header), Self> {
        let Message { header, content } = self;
        match content {
            Some(body) => match body.try_cast() {
                Ok(value) => Ok((value, *header)),
                Err(body) => Err(Self::from_raw_parts(header, Some(body))),
            },
            None => Err(Self::from_raw_parts(header, None)),
        }
    }

    /// Tries to clone the message. This operation fails if the body is not clonable
    #[must_use]
    pub fn try_clone(&self) -> Option<Self> {
        Some(Self {
            header: self.header.clone(),
            content: match &self.content {
                Some(body) => Some(body.try_clone()?),
                None => None,
            },
        })
    }
}

impl Clone for Message {
    fn clone(&self) -> Self {
        self.try_clone()
            .expect("expected clonable message body: value not clonable")
    }
}

// # Display

impl Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Message {{ {} bytes {:?}  }}",
            self.length(),
            self.content
        )
    }
}

// SAFTY:
// A message only contains primitve data, ptrs that are threadsafe
// and a untyped contained value.
unsafe impl Send for Message {}

impl UnwindSafe for Message {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::type_name;

    #[test]
    fn message_fmt() {
        let msg = Message::default()
            .id(123)
            .src([1; 6])
            .dst([2; 6])
            .with_content(String::from("Hello world!"));

        #[cfg(debug_assertions)]
        assert_eq!(
            msg.to_string(),
            format!(
                "Message {{ 76 bytes Some(Body {{ length: 12, type: {:?}, value: \"Hello world!\" }})  }}",
                type_name::<String>()
            )
        );

        assert!(msg.can_cast::<String>());
        assert_eq!(msg.content::<String>(), "Hello world!");
    }

    #[test]
    fn message_cast() {
        #[derive(Debug, Clone)]
        struct A(i32);
        impl MessageBody for A {
            fn byte_len(&self) -> usize {
                0
            }
        }

        let msg = Message::default()
            .id(123)
            .receiver_module_id(ModuleId(1))
            .sender_module_id(ModuleId(2))
            .with_content(A(42));
        let (value, header) = msg.cast::<A>();
        assert_eq!(header.id, 123);
        assert_eq!(value.0, 42);
    }
}
