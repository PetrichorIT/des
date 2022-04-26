use std::{
    any::TypeId,
    cell::UnsafeCell,
    fmt::Debug,
    hash::Hash,
    marker::{PhantomData, Unsize},
    ops::{CoerceUnsized, Deref, DerefMut, DispatchFromDyn},
    rc::{Rc, Weak},
};

// TODO: Do with dyn Any
#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PtrWeakVoid {
    inner: PtrWeak<(), Mut>,
    type_id: TypeId,
}

#[allow(unused)]
impl PtrWeakVoid {
    pub fn new<T: 'static + Sized>(value: PtrWeak<T, Mut>) -> Self {
        assert!(std::mem::size_of::<PtrWeak<T, Mut>>() == 8);

        // SAFTY:
        // Transmute used for abstracting inner workings of Rc away
        // and to implement own version of dyn dispatch.
        let this = unsafe { std::mem::transmute::<PtrWeak<T, Mut>, PtrWeak<(), Mut>>(value) };
        Self {
            inner: this,
            type_id: TypeId::of::<T>(),
        }
    }

    pub(crate) fn is<T: 'static + Sized>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    pub(crate) fn downcast<T: 'static + Sized>(self) -> Option<PtrWeak<T, Mut>> {
        if self.is::<T>() {
            // SAFTY:
            // Transmute used for abstracting inner workings of Rc away
            // and to implement own version of dyn dispatch.
            let value = self.inner;
            Some(unsafe { std::mem::transmute::<PtrWeak<(), Mut>, PtrWeak<T, Mut>>(value) })
        } else {
            None
        }
    }
}

use self::private::MutabilityState;
mod private {
    pub trait MutabilityState {}
}

/// A [Ptr] that does not support mutation ([Const]).
pub type PtrConst<T> = Ptr<T, Const>;
/// A [Ptr] that does support mutation ([Mut]).
pub type PtrMut<T> = Ptr<T, Mut>;
/// A [PtrWeak] that does not support mutation ([Const]).
pub type PtrWeakConst<T> = PtrWeak<T, Const>;
/// A [PtrWeak] that does support mutation ([Mut]).
pub type PtrWeakMut<T> = PtrWeak<T, Mut>;

///
/// An annotation type that indicates a [Ptr] or a [PtrWeak]
/// does not support mutation.
///
#[derive(Debug)]
pub struct Const;
impl private::MutabilityState for Const {}

///
/// An annotation type that indicates a [Ptr] or a [PtrWeak]
/// does support mutation.
///
#[derive(Debug)]
pub struct Mut;
impl private::MutabilityState for Mut {}

///
/// A wrapper to a [Rc] that allows interior mutability
/// in a uncontrolled manner.
///
/// # Safty contract
///
/// This type should not be used with container types
/// since having it allows simultanios &T and &mut T borrows.
/// Because of that the integrity of refences into a container
/// can not be guranteed.
///
/// As a quick rule of thumb do not store the references
/// returned by [Deref] or [DerefMut] anywhere.
///
pub struct Ptr<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    inner: Rc<UnsafeCell<T>>,
    _phantom: PhantomData<S>,
}

impl<T, S> Ptr<T, S>
where
    S: MutabilityState,
{
    ///
    /// Constructs a new [Ptr<T>].
    ///
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(UnsafeCell::new(value)),
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Ptr<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    ///
    /// Gets the number of strong ([Ptr])
    /// pointers to the underlying allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// let the_solution: PtrMut<usize> = Ptr::new(42);
    /// let _copy_cat = Ptr::clone(&the_solution);
    ///
    /// assert_eq!(Ptr::strong_count(&the_solution), 2);
    /// #
    /// # drop(the_solution);
    /// # drop(_copy_cat);
    /// ```
    pub fn strong_count(this: &Self) -> usize {
        Rc::strong_count(&this.inner)
    }

    ///
    /// Gets the number of weak ([PtrWeak])
    /// pointer to the underlying allocation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use des::prelude::*;
    /// #
    /// let pw: PtrConst<String> = Ptr::new("SuperSecretPW".to_string());
    /// let _weak = PtrWeak::from_strong(&pw);
    ///
    /// assert_eq!(Ptr::weak_count(&pw), 1);
    /// #
    /// # drop(pw);
    /// # drop(_weak);
    /// ```
    pub fn weak_count(this: &Self) -> usize {
        Rc::weak_count(&this.inner)
    }
}

impl<T: ?Sized> Ptr<T, Mut> {
    ///
    /// Declares a reference as read-only. Not reversable.
    ///
    pub fn make_const(self) -> Ptr<T, Const> {
        Ptr {
            inner: self.inner,
            _phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> Ptr<T, Const> {
    ///
    /// Only for internal use
    ///
    #[allow(unused)]
    pub(crate) fn force_mutable(self) -> Ptr<T, Mut> {
        Ptr {
            inner: self.inner,
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Debug for Ptr<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ptr")
            .field("value", &"Some(_)")
            .field("strong_count", &Self::strong_count(self))
            .field("weak_count", &Self::weak_count(self))
            .finish()
    }
}

impl<T, S> AsRef<T> for Ptr<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn as_ref(&self) -> &T {
        // SAFTY:
        // This deref in considered safe since it only extends Ptr
        // with the default Rc behaviour
        unsafe { &*self.inner.as_ref().get() }
    }
}

impl<T, S> Clone for Ptr<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Deref for Ptr<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T> DerefMut for Ptr<T, Mut>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut T {
        // SAFTY:
        // This can be considered a valid extension of the safty contract
        // acording to the type definition
        unsafe { &mut *self.inner.as_ref().get() }
    }
}

impl<T, S, U> CoerceUnsized<Ptr<U, S>> for Ptr<T, S>
where
    T: ?Sized + Unsize<U>,
    S: MutabilityState,
    U: ?Sized,
{
}

impl<T, S, U> DispatchFromDyn<Ptr<U, S>> for Ptr<T, S>
where
    T: ?Sized + Unsize<U>,
    S: MutabilityState,
    U: ?Sized,
{
}

impl<T, S> PartialEq for Ptr<T, S>
where
    T: PartialEq,
    S: MutabilityState,
{
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T, S> Eq for Ptr<T, S>
where
    T: Eq,
    S: MutabilityState,
{
}

impl<T, S> Hash for Ptr<T, S>
where
    T: Hash,
    S: MutabilityState,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<T, S> PartialOrd for Ptr<T, S>
where
    T: PartialOrd,
    S: MutabilityState,
{
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.deref().partial_cmp(rhs.deref())
    }
}

impl<T, S> Ord for Ptr<T, S>
where
    T: Ord,
    S: MutabilityState,
{
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.deref().cmp(rhs.deref())
    }
}

// WEAK

///
/// A wrapper to a [Weak] that allows interior mutability
/// in a uncontrolled manner.
///
/// # Safty contract
///
/// This type should not be used with container types
/// since having it allows simultanios &T and &mut T borrows.
/// Because of that the integrity of refences into a container
/// can not be guranteed.
///
/// As a quick rule of thumb do not store the references
/// returned by [Deref] or [DerefMut] anywhere.
///
pub struct PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    inner: Weak<UnsafeCell<T>>,
    _phantom: PhantomData<S>,
}

impl<T, S> PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    ///
    /// Creates a new empty instance of [PtrWeak].
    ///
    pub fn new() -> Self
    where
        T: Sized,
    {
        Self {
            inner: Weak::new(),
            _phantom: PhantomData,
        }
    }

    ///
    /// Constructs a new [PtrWeak<T>] from a [Ptr<T>].
    ///
    pub fn from_strong(ptr: &Ptr<T, S>) -> Self {
        Self {
            inner: Rc::downgrade(&ptr.inner),
            _phantom: PhantomData,
        }
    }

    ///
    /// Gets the number of strong ([Ptr]) pointers pointing to this allocation.
    ///
    /// If self was created using [PtrWeak::new], this will return 0.
    ///
    pub fn strong_count(&self) -> usize {
        self.inner.strong_count()
    }

    ///
    /// Gets the number of [PtrWeak] pointers pointing to this allocation.
    ///
    /// If no strong pointers remain, this will return zero.
    ///
    pub fn weak_count(&self) -> usize {
        self.inner.weak_count()
    }
}

impl<T: ?Sized> PtrWeak<T, Mut> {
    ///
    /// Declares a reference as read-only. Not reversable.
    ///
    pub fn make_const(self) -> PtrWeak<T, Const> {
        PtrWeak {
            inner: self.inner,
            _phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> PtrWeak<T, Const> {
    ///
    /// Only for internal use
    ///
    #[allow(unused)]
    pub(crate) fn force_mutable(self) -> PtrWeak<T, Mut> {
        PtrWeak {
            inner: self.inner,
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Default for PtrWeak<T, S>
where
    S: MutabilityState,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S> Debug for PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtrWeak")
            .field(
                "value",
                if self.inner.upgrade().is_some() {
                    &"Some(_)"
                } else {
                    &"None"
                },
            )
            .field("strong_count", &self.inner.strong_count())
            .field("weak_count", &self.inner.weak_count())
            .finish()
    }
}

impl<T, S> AsRef<T> for PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn as_ref(&self) -> &T {
        // SAFTY:
        // This deref in considered safe since it only extends Ptr
        // with the default Rc behaviour
        unsafe { &*self.inner.upgrade().as_ref().unwrap().get() }
    }
}

impl<T, S> Clone for PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Deref for PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T> DerefMut for PtrWeak<T, Mut>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut T {
        // SAFTY:
        // This can be considered a valid extension of the safty contract
        // acording to the type definition
        unsafe { &mut *self.inner.upgrade().as_ref().unwrap().as_ref().get() }
    }
}

impl<T, S, U> CoerceUnsized<PtrWeak<U, S>> for PtrWeak<T, S>
where
    T: ?Sized + Unsize<U>,
    S: MutabilityState,
    U: ?Sized,
{
}

impl<T, S, U> DispatchFromDyn<PtrWeak<U, S>> for PtrWeak<T, S>
where
    T: ?Sized + Unsize<U>,
    S: MutabilityState,
    U: ?Sized,
{
}

impl<T, S> PartialEq for PtrWeak<T, S>
where
    T: PartialEq,
    S: MutabilityState,
{
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T, S> Eq for PtrWeak<T, S>
where
    T: Eq,
    S: MutabilityState,
{
}

impl<T, S> Hash for PtrWeak<T, S>
where
    T: Hash,
    S: MutabilityState,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<T, S> PartialOrd for PtrWeak<T, S>
where
    T: PartialOrd,
    S: MutabilityState,
{
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.deref().partial_cmp(rhs.deref())
    }
}

impl<T, S> Ord for PtrWeak<T, S>
where
    T: Ord,
    S: MutabilityState,
{
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.deref().cmp(rhs.deref())
    }
}

impl<T, S> From<&Ptr<T, S>> for PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn from(ptr: &Ptr<T, S>) -> Self {
        Self::from_strong(ptr)
    }
}

impl<T, S> From<Ptr<T, S>> for PtrWeak<T, S>
where
    T: ?Sized,
    S: MutabilityState,
{
    fn from(ptr: Ptr<T, S>) -> Self {
        Self::from_strong(&ptr)
    }
}
