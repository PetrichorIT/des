use std::{
    any::TypeId,
    cell::UnsafeCell,
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

pub type PtrConst<T> = Ptr<T, Const>;
pub type PtrMut<T> = Ptr<T, Mut>;
pub type PtrWeakConst<T> = PtrWeak<T, Const>;
pub type PtrWeakMut<T> = PtrWeak<T, Mut>;

#[derive(Debug)]
pub struct Const;
impl private::MutabilityState for Const {}

#[derive(Debug)]
pub struct Mut;
impl private::MutabilityState for Mut {}

#[derive(Debug)]
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

#[derive(Debug)]
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
    /// Constructs a new [PtrWeak<T>] from a [Ptr<T>].
    ///
    pub fn from_strong(ptr: &Ptr<T, S>) -> Self {
        Self {
            inner: Rc::downgrade(&ptr.inner),
            _phantom: PhantomData,
        }
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
