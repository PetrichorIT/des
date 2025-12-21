//! Generic network messages.
//!
//! Traditionally modules communicate via messages through the simulated fabric.
//! This module contains the abstract [`Message` type](Message) that represents arbitrary
//! payloads and the APIs to send messages. The receiving of messages is handled
//! by [`module`](crate::net::module).
//!
//! See [`Message`], [`Header`] and [`Body`] to learn about the creation and usage of messages
//! as objects. Use the functions [`send`], [`send_at`] and [`send_in`] to send messages onto
//! gate chains, to communicate with other modules. Schedule messages directed at yourself
//! using [`schedule_at`] and [`schedule_in`].

use crate::net::gate::GateRef;
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

mod extension;
pub use extension::*;

///
/// A network message holding a arbitrary payload.
///
/// A message is composed from two parts:
/// - a `Header` containing generic message parameters
/// - and a optional `Body`, containing an arbitrary payload.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
#[derive(Debug)]
#[must_use]
pub struct Message {
    /// The header contained in the message.
    pub header: Box<Header>,
    /// The body contained in the message. Default is ().
    pub body: Body,
    /// The extensions attached to the message.
    pub extensions: Extensions,
}

impl Message {
    /// Constructs a message from its raw parts.
    ///
    /// The header is boxed for improved internal layout.
    pub fn from_raw_parts(header: Box<Header>, body: Option<Body>, extensions: Extensions) -> Self {
        Self {
            header,
            body: body.unwrap_or(Body::empty()),
            extensions,
        }
    }

    /// From parts
    pub fn from_parts<T: MessageBody + Any + Clone + Debug>(
        header: Header,
        body: Option<T>,
    ) -> Self {
        Self::from_raw_parts(Box::new(header), body.map(Body::new), Extensions::default())
    }

    /// Returns the length of the complete message.
    ///
    /// The length is the sum of the bodys length and a fixed header length.
    #[must_use]
    pub fn length(&self) -> usize {
        self.body.length() + self.header.byte_len()
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            header: Box::new(Header::default()),
            body: Body::empty(),
            extensions: Extensions::default(),
        }
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
    pub fn with_id(mut self, id: MessageId) -> Self {
        self.header.id = id;
        self
    }

    /// **Builder** that sets the messsage kind field.
    pub fn with_kind(mut self, kind: MessageKind) -> Self {
        self.header.kind = kind;
        self
    }

    /// **Builder** that sets the creation time field.
    pub fn with_creation_time(mut self, time: SimTime) -> Self {
        self.header.creation_time = time;
        self
    }

    /// **Builder** that sets the send time field.
    pub fn with_send_time(mut self, time: SimTime) -> Self {
        self.header.send_time = time;
        self
    }

    /// **Builder** that sets the last gate field.
    pub fn with_last_gate(mut self, gate: GateRef) -> Self {
        self.header.last_gate = Some(gate);
        self
    }

    /// **Builder** that sets the source MAC address field.
    pub fn with_src(mut self, src: [u8; 6]) -> Self {
        self.header.src = src;
        self
    }

    /// **Builder** that sets the destination MAC address field.
    pub fn with_dst(mut self, dest: [u8; 6]) -> Self {
        self.header.dst = dest;
        self
    }
}

// # Content Accessing

impl Message {
    /// Sets the content of the message.
    pub fn set_body(&mut self, body: Body) {
        self.body = body;
    }

    /// Sets the content of the message.
    #[inline]
    pub fn set_content<T>(&mut self, value: T)
    where
        T: MessageBody + Clone + Debug + Any,
    {
        self.set_body(Body::new(value));
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

    /// Performs a [`try_into_content`](Message::try_into_content) unwraping the result.
    ///
    /// # Panics
    ///
    /// Panics if he cast fails.
    #[must_use]
    pub fn into_content<T: 'static + MessageBody + Send>(self) -> (T, Header, Extensions) {
        self.try_into_content().expect("could not cast to type T")
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
    pub fn try_into_content<T: 'static + MessageBody + Send>(
        self,
    ) -> Result<(T, Header, Extensions), Self> {
        let Message {
            header,
            body,
            extensions,
        } = self;
        match body.try_into_content() {
            Ok(value) => Ok((value, *header, extensions)),
            Err(body) => Err(Self::from_raw_parts(header, Some(body), extensions)),
        }
    }

    /// Tries to clone the message. This operation fails if the body is not clonable
    #[must_use]
    pub fn try_clone(&self) -> Option<Self> {
        Some(Self {
            header: self.header.clone(),
            body: self.body.try_clone()?,
            extensions: Extensions::default(),
        })
    }
}

//
// # Extensions
//

impl Message {
    /// Overrides the extension set of the message.
    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }

    /// Adds an extension to the message.
    pub fn with_extension<T: Any + Send>(mut self, extension: T) -> Self {
        self.extensions.set(extension);
        self
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
        write!(f, "Message {{ {} bytes {:?}  ", self.length(), self.body)?;
        if !self.extensions.is_empty() {
            write!(f, "+ {:?} ", self.extensions)?;
        }

        write!(f, "}}")
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

    macro_rules! test_primive {
        ($ident:ident { $($e:expr => $s:expr),+ }) => {
            #[test]
            #[allow(unused_allocation)]
            fn $ident() {
                $(
                    assert_eq!(($e).byte_len(), $s);
                )+
            }
        };
    }

    test_primive!(body_size_int {
        32u8 => 1,
        32u16 => 2,
        32u32 => 4,
        32u64 => 8,
        32u128 => 16,
        -32i8 => 1,
        -32i16 => 2,
        -32i32 => 4,
        -32i64 => 8,
        -32i128 => 16
    });

    test_primive!(body_size_float {
        0.1f32 => 4,
        0.45f64 => 8
    });

    test_primive!(body_size_other_primitives {
        () => 0,
        true => 1,
        'b' => 4
    });

    test_primive!(body_size_string {
        String::new() => 0,
        "Hello World".to_string() => 11,
        "Hello World😀".to_string() => 15
    });

    test_primive!(body_size_boxed {
        Box::new(0u8) => 1,
        Box::new(0i128) => 16,
        Box::new(String::from("Hello World")) => 11,
        Box::new(()) => 0
    });

    test_primive!(body_size_option {
        Some(0u8) => 1,
        Option::<u8>::None => 0,
        Some("Hello World".to_string()) => 11,
        Option::<String>::None => 0
    });

    test_primive!(body_size_result {
       Result::<_, u8>::Ok("Hello World".to_string()) => 11,
       Result::<_, u8>::Ok(String::new()) => 0,
       Result::<String, _>::Err(0u8) => 1,
       Result::<String, _>::Err(16u8) => 1
    });

    test_primive!(body_size_collection {
        vec![1, 2, 3u8] => 3,
        vec![String::new(), "Hello World".to_string(), "ABC".to_string()] => 11 + 3
    });

    #[test]
    fn display() {
        let msg = Message::default()
            .with_id(123)
            .with_src([1; 6])
            .with_dst([2; 6])
            .with_content(String::from("Hello world!"));

        #[cfg(debug_assertions)]
        assert_eq!(
            msg.to_string(),
            format!(
                "Message {{ 76 bytes Body {{ length: 12, type: {:?}, value: \"Hello world!\" }}  }}",
                std::any::type_name::<String>()
            )
        );

        assert!(msg.body.is::<String>());
        assert_eq!(msg.body.content::<String>(), "Hello world!");
    }

    #[test]
    fn cast() {
        #[derive(Debug, Clone)]
        struct A(i32);
        impl MessageBody for A {
            fn byte_len(&self) -> usize {
                0
            }
        }

        let msg = Message::default().with_id(123).with_content(A(42));

        let (value, header, _) = msg.into_content::<A>();
        assert_eq!(header.id, 123);
        assert_eq!(value.0, 42);
    }

    #[derive(Debug, PartialEq, Eq)]
    struct PrivateType;

    #[test]
    fn extensions() {
        let mut msg = Message::default();

        msg.extensions.set(String::from("hello world"));
        msg.extensions.set(PrivateType);

        assert_eq!(msg.extensions.get::<String>(), Some(&"hello world".into()));
        assert_eq!(msg.extensions.get::<PrivateType>(), Some(&PrivateType));
    }

    #[test]
    fn extensions_display() {
        let mut msg = Message::default();

        msg.extensions.set(String::from("hello world"));
        msg.extensions.set(PrivateType);

        #[cfg(not(debug_assertions))]
        assert!(
            msg.to_string()
                .contains(&format!("{:?}", std::any::TypeId::of::<String>())),
        );
        #[cfg(not(debug_assertions))]
        assert!(
            msg.to_string()
                .contains(&format!("{:?}", std::any::TypeId::of::<PrivateType>()))
        );

        #[cfg(debug_assertions)]
        assert!(msg.to_string().contains(std::any::type_name::<String>()));
        #[cfg(debug_assertions)]
        assert!(
            msg.to_string()
                .contains(std::any::type_name::<PrivateType>())
        );
    }

    #[test]
    fn extensions_clone() {
        let mut msg = Message::default();

        msg.extensions.set(String::from("hello world"));
        msg.extensions.set(PrivateType);

        let cloned_msg = msg.clone();
        assert!(cloned_msg.extensions.is_empty());
    }
}
