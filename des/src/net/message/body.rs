/// A trait that allows a type to be mesured in bits / bytes.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait MessageBody {
    /// The length of the message body in bytes.
    fn byte_len(&self) -> usize;
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

msg_body_lenable!(&'static str);
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

impl<K: MessageBody, V: MessageBody, S> MessageBody for HashMap<K, V, S> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for (k, v) in self {
            sum += k.byte_len() + v.byte_len();
        }
        sum
    }
}

impl<K: MessageBody, V: MessageBody> MessageBody for BTreeMap<K, V> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for (k, v) in self {
            sum += k.byte_len() + v.byte_len();
        }
        sum
    }
}

impl<T: MessageBody, S> MessageBody for HashSet<T, S> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for v in self {
            sum += v.byte_len();
        }
        sum
    }
}

impl<T: MessageBody> MessageBody for BTreeSet<T> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for v in self {
            sum += v.byte_len();
        }
        sum
    }
}

impl<T: MessageBody> MessageBody for BinaryHeap<T> {
    fn byte_len(&self) -> usize {
        let mut sum = 0;
        for v in self {
            sum += v.byte_len();
        }
        sum
    }
}

// # std::net
use std::net;

impl MessageBody for net::Ipv4Addr {
    fn byte_len(&self) -> usize {
        4
    }
}

impl MessageBody for net::Ipv6Addr {
    fn byte_len(&self) -> usize {
        16
    }
}

impl MessageBody for net::IpAddr {
    fn byte_len(&self) -> usize {
        match self {
            Self::V4(v4) => v4.byte_len(),
            Self::V6(v6) => v6.byte_len(),
        }
    }
}

impl MessageBody for net::SocketAddrV4 {
    fn byte_len(&self) -> usize {
        4 + 2
    }
}

impl MessageBody for net::SocketAddrV6 {
    fn byte_len(&self) -> usize {
        16 + 2
    }
}

impl MessageBody for net::SocketAddr {
    fn byte_len(&self) -> usize {
        match self {
            Self::V4(v4) => v4.byte_len(),
            Self::V6(v6) => v6.byte_len(),
        }
    }
}

// # Time
use crate::time;

impl MessageBody for time::Duration {
    fn byte_len(&self) -> usize {
        16
    }
}

impl MessageBody for time::SimTime {
    fn byte_len(&self) -> usize {
        16
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

///
/// A message body that does mimics a custom size
/// independet of actualy size.
///
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CustomSizeBody<T> {
    byte_len: usize,
    inner: T,
}

impl<T> CustomSizeBody<T> {
    ///
    /// Creates a new instance of `Self`.
    ///
    #[must_use]
    pub fn new(byte_len: usize, inner: T) -> Self {
        Self { byte_len, inner }
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
        self.byte_len
    }
}

#[cfg(test)]
mod tests {
    use super::{super::*, *};

    #[test]
    fn custom_message_body() {
        let slice = "Hello world!";

        let mut body = CustomSizeBody::new(16, slice);
        assert_eq!(body.inner(), &"Hello world!");
        assert_eq!(body.inner_mut(), &mut "Hello world!");

        let msg = Message::new().content(body).build();
        assert_eq!(msg.length(), 16 + 64);

        let content = msg.content::<CustomSizeBody<&str>>();
        assert_eq!((*content.inner()).as_ptr(), slice.as_ptr());

        let content = content.clone();
        assert_eq!(content.into_inner(), "Hello world!");
    }

    #[test]
    fn auto_impl() {
        assert_eq!(
            [1, 2, 3, 4u8]
                .into_iter()
                .collect::<VecDeque<_>>()
                .byte_len(),
            4
        );

        assert_eq!(
            [1, 2, 3, 4u8]
                .into_iter()
                .collect::<LinkedList<_>>()
                .byte_len(),
            4
        );

        assert_eq!(
            [1, 2, 3, 4u8]
                .into_iter()
                .collect::<HashSet<_>>()
                .byte_len(),
            4
        );

        assert_eq!(
            [1, 2, 3, 4u8]
                .into_iter()
                .collect::<BTreeSet<_>>()
                .byte_len(),
            4
        );

        assert_eq!(
            [1, 2, 3, 4u8]
                .into_iter()
                .collect::<BinaryHeap<_>>()
                .byte_len(),
            4
        );

        assert_eq!(
            [(1, 1), (2, 2), (3, 3), (4u8, 4u16)]
                .into_iter()
                .collect::<HashMap<_, _>>()
                .byte_len(),
            12
        );

        assert_eq!(
            [(1, 1), (2, 2), (3, 3), (4u8, 4u16)]
                .into_iter()
                .collect::<BTreeMap<_, _>>()
                .byte_len(),
            12
        );

        assert_eq!((&[1, 2, 3, 4u8][..3]).byte_len(), 3);

        assert_eq!(net::Ipv4Addr::new(1, 2, 3, 4).byte_len(), 4);
        assert_eq!(net::Ipv6Addr::new(1, 2, 3, 4, 0, 0, 0, 0).byte_len(), 16);
        assert_eq!(
            net::IpAddr::V4(net::Ipv4Addr::new(1, 2, 3, 4)).byte_len(),
            4
        );
        assert_eq!(
            net::IpAddr::V6(net::Ipv6Addr::new(1, 2, 3, 4, 0, 0, 0, 0)).byte_len(),
            16
        );

        assert_eq!(
            net::SocketAddrV4::new(net::Ipv4Addr::new(1, 2, 3, 4), 0).byte_len(),
            4 + 2
        );
        assert_eq!(
            net::SocketAddrV6::new(net::Ipv6Addr::new(1, 2, 3, 4, 0, 0, 0, 0), 0, 0, 0).byte_len(),
            16 + 2
        );
        assert_eq!(
            net::SocketAddr::V4(net::SocketAddrV4::new(net::Ipv4Addr::new(1, 2, 3, 4), 0))
                .byte_len(),
            4 + 2
        );
        assert_eq!(
            net::SocketAddr::V6(net::SocketAddrV6::new(
                net::Ipv6Addr::new(1, 2, 3, 4, 0, 0, 0, 0),
                0,
                0,
                0
            ))
            .byte_len(),
            16 + 2
        );

        assert_eq!(time::Duration::from_secs(123).byte_len(), 16);
        assert_eq!(
            time::SimTime::from_duration(time::Duration::from_secs(123)).byte_len(),
            16
        );
    }
}
