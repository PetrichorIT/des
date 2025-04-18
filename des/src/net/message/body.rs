use std::{
    any::{type_name, Any, TypeId},
    fmt::{self, Debug},
    mem,
    ptr::null_mut,
};

/// A message body, which stores an arbitrary value, potentially cloneable and debuggable.
pub struct Body {
    data: *mut (),
    length: usize,
    vtable: &'static VTable,
}

impl Body {
    /// Creates a new message body, using a cloneable and debuggable value.
    pub fn new<T>(value: T) -> Self
    where
        T: MessageBody + Any + Clone + Debug,
    {
        let length = value.byte_len();
        let boxed = Box::new(value);
        Self {
            data: Box::into_raw(boxed).cast(),
            length,
            vtable: vtable::<T>(),
        }
    }

    /// Creates a new message body, using a cloneable and debuggable value, with a specified length.
    pub fn new_with_len<T>(value: T, length: usize) -> Self
    where
        T: Any + Clone + Debug,
    {
        let boxed = Box::new(value);
        Self {
            data: Box::into_raw(boxed).cast(),
            length,
            vtable: vtable::<T>(),
        }
    }

    /// Creates a new message body, using a non-cloneable, but debuggable value.
    ///
    /// Calls to`Message::clone` / `Body::clone` will panic.
    pub fn new_non_clonable<T>(value: T) -> Self
    where
        T: MessageBody + Any + Debug,
    {
        let length = value.byte_len();
        let boxed = Box::new(value);
        Self {
            data: Box::into_raw(boxed).cast(),
            length,
            vtable: vtable_non_clonable::<T>(),
        }
    }

    /// Creates a new message body, using a cloneable, but non-debuggable value.
    ///
    /// The trait `Debug` is still implemented for `Body`, but will not show any
    /// information about the inner value.
    pub fn new_non_debugable<T>(value: T) -> Self
    where
        T: Any + Clone,
    {
        let boxed = Box::new(value);
        Self {
            data: Box::into_raw(boxed).cast(),
            length: mem::size_of::<T>(),
            vtable: vtable_non_debugable::<T>(),
        }
    }

    /// The length of the message body.
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Tests which inner type is stored in the message body.
    ///
    /// See also `Any::is`.
    #[must_use]
    pub fn is<T: Any>(&self) -> bool {
        let id = unsafe { (self.vtable.type_id)() };
        id == TypeId::of::<T>()
    }

    /// Tries to cast the message body to the given type.
    ///
    /// See also `Any::downcast`.
    ///
    /// # Errors
    ///
    /// If the contained type is not `T`, this function returns `self` unchanged.
    pub fn try_cast<T: Any>(mut self) -> Result<T, Self> {
        if self.is::<T>() {
            // take the ptr so that drop does not do shit
            let boxed =
                unsafe { Box::from_raw(mem::replace(&mut self.data, null_mut()).cast::<T>()) };
            Ok(*boxed)
        } else {
            Err(self)
        }
    }

    /// Tries to cast the message body as a reference to the given type.
    ///
    /// See also `Any::downcast_ref`.
    #[must_use]
    pub fn try_content<T: Any>(&self) -> Option<&T> {
        self.is::<T>().then(|| unsafe { &*self.data.cast::<T>() })
    }

    /// Tries to cast the message body as a mutable reference to the given type.
    ///
    /// See also `Any::downcast_mut`.
    pub fn try_content_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.is::<T>()
            .then(|| unsafe { &mut *self.data.cast::<T>() })
    }

    /// Tries to clone the body. This operation fails if the inner type `T`
    /// is not cloneable
    #[must_use]
    pub fn try_clone(&self) -> Option<Self> {
        let cloned = unsafe { (self.vtable.try_clone)(self.data) };
        cloned.map(|data| Self {
            data,
            length: self.length,
            vtable: self.vtable,
        })
    }
}

impl Clone for Body {
    fn clone(&self) -> Self {
        self.try_clone()
            .expect("expected contained value to be cloneable")
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body")
            .field("length", &self.length)
            .field("type", &unsafe { (self.vtable.type_name)() })
            .field(
                "value",
                &DebugPrinter {
                    ptr: self.data,
                    f: self.vtable.debug,
                },
            )
            .finish()
    }
}

impl Drop for Body {
    fn drop(&mut self) {
        unsafe { (self.vtable.drop)(self.data) }
    }
}

struct DebugPrinter {
    ptr: *mut (),
    f: unsafe fn(*const (), &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl Debug for DebugPrinter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { (self.f)(self.ptr, f) }
    }
}

struct VTable {
    type_id: unsafe fn() -> TypeId,
    type_name: unsafe fn() -> &'static str,
    debug: unsafe fn(*const (), &mut fmt::Formatter<'_>) -> fmt::Result,
    try_clone: unsafe fn(*const ()) -> Option<*mut ()>,
    drop: unsafe fn(*mut ()),
}

fn vtable<T: Any + Debug + Clone>() -> &'static VTable {
    &VTable {
        type_id: vtype_id::<T>,
        type_name: vtype_name::<T>,
        debug: vdebug::<T>,
        try_clone: vclone::<T>,
        drop: vdrop::<T>,
    }
}

fn vtable_non_clonable<T: Any + Debug>() -> &'static VTable {
    &VTable {
        type_id: vtype_id::<T>,
        type_name: vtype_name::<T>,
        debug: vdebug::<T>,
        try_clone: vclone_panic,
        drop: vdrop::<T>,
    }
}

fn vtable_non_debugable<T: Any + Clone>() -> &'static VTable {
    &VTable {
        type_id: vtype_id::<T>,
        type_name: vtype_name::<T>,
        debug: vdebug_unknown,
        try_clone: vclone::<T>,
        drop: vdrop::<T>,
    }
}

unsafe fn vtype_id<T: Any>() -> TypeId {
    TypeId::of::<T>()
}

unsafe fn vtype_name<T>() -> &'static str {
    type_name::<T>()
}

unsafe fn vdebug<T: Debug>(ptr: *const (), f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let value = unsafe { &*ptr.cast::<T>() };
    value.fmt(f)
}

unsafe fn vdebug_unknown(_: *const (), f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "?")
}

unsafe fn vclone<T: Clone>(ptr: *const ()) -> Option<*mut ()> {
    let value = T::clone(unsafe { &*ptr.cast::<T>() });
    Some(Box::into_raw(Box::new(value)).cast())
}

unsafe fn vclone_panic(_: *const ()) -> Option<*mut ()> {
    None
}

unsafe fn vdrop<T>(ptr: *mut ()) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(&mut *ptr.cast::<T>()));
        }
    }
}

/// A trait that allows a type to be mesured in bits / bytes.
///
/// * This type is only available of DES is build with the `"net"` feature.*
#[cfg_attr(doc_cfg, doc(cfg(feature = "net")))]
pub trait MessageBody {
    /// The length of the message body in bytes.
    fn byte_len(&self) -> usize;
}

// # Primitives

macro_rules! msg_body_from_mem_size {
    ($($t: ty),*) => {
        $(
            impl MessageBody for $t {
                fn byte_len(&self) -> usize {
                    std::mem::size_of::<Self>()
                }
            }
        )*

    };
}

msg_body_from_mem_size!(
    (),
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    bool,
    char
);

macro_rules! msg_body_from_len {
    ($t: ty) => {
        impl MessageBody for $t {
            fn byte_len(&self) -> usize {
                self.len()
            }
        }
    };
}

msg_body_from_len!(&'static str);
msg_body_from_len!(String);

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

macro_rules! msg_body_for_tupels {
    ( $( $name:ident ),+ ) => {
        impl<$($name: MessageBody),+> MessageBody for ($($name,)+)
        {
            #[allow(non_snake_case)]
            fn byte_len(&self) -> usize {
                let ($($name,)+) = self;
                $($name.byte_len() +)+0
            }
        }
    };
}

msg_body_for_tupels!(A);
msg_body_for_tupels!(A, B);
msg_body_for_tupels!(A, B, C);
msg_body_for_tupels!(A, B, C, D);
msg_body_for_tupels!(A, B, C, D, E);
msg_body_for_tupels!(A, B, C, D, E, F);
msg_body_for_tupels!(A, B, C, D, E, F, G);
msg_body_for_tupels!(A, B, C, D, E, F, G, H);
msg_body_for_tupels!(A, B, C, D, E, F, G, H, I);
msg_body_for_tupels!(A, B, C, D, E, F, G, H, I, J);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_maintains_type_identity() {
        assert!(Body::new("Hello world!").is::<&str>());
        assert!(!Body::new("Hello world!").is::<String>());

        assert!(Body::new("Hello world!".to_string()).is::<String>());
        assert!(!Body::new("Hello world!".to_string()).is::<&str>());

        assert!(Body::new(1u8).is::<u8>());
        assert!(!Body::new(1u8).is::<usize>());

        assert!(Body::new_non_debugable(true).is::<bool>());
        assert!(!Body::new_non_debugable(true).is::<u8>());

        assert!(Body::new_non_clonable('a').is::<char>());
        assert!(!Body::new_non_clonable('a').is::<u8>());
    }

    #[test]
    fn body_allows_downcasting() {
        assert_eq!(
            Body::new("Hello world!").try_cast::<&str>().unwrap(),
            "Hello world!"
        );
        assert_eq!(Body::new(42u8).try_cast::<u8>().unwrap(), 42);
        assert_eq!(Body::new(true).try_cast::<bool>().unwrap(), true);

        assert_eq!(
            Body::new("Hello world!").try_content::<&str>().unwrap(),
            &"Hello world!"
        );
        assert_eq!(Body::new(42u8).try_content::<u8>().unwrap(), &42);
        assert_eq!(Body::new(true).try_content::<bool>().unwrap(), &true);

        assert_eq!(
            Body::new("Hello world!").try_content_mut::<&str>().unwrap(),
            &mut "Hello world!"
        );
        assert_eq!(Body::new(42u8).try_content_mut::<u8>().unwrap(), &mut 42);
        assert_eq!(
            Body::new(true).try_content_mut::<bool>().unwrap(),
            &mut true
        );
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
