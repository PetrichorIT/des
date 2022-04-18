//!
//!  Memory management
//!

use std::{
    any::TypeId,
    cell::UnsafeCell,
    hash::Hash,
    marker::{PhantomData, Unsize},
    ops::{CoerceUnsized, Deref, DerefMut, DispatchFromDyn},
    rc::Rc,
};

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntypedMrc {
    inner: MrcS<(), Mutable>,
    type_id: TypeId,
}

#[allow(unused)]
impl UntypedMrc {
    pub fn new<T: 'static + Sized>(value: MrcS<T, Mutable>) -> Self {
        assert!(std::mem::size_of::<MrcS<T, Mutable>>() == 8);

        // SAFTY:
        // Transmute used for abstracting inner workings of Rc away
        // and to implement own version of dyn dispatch.
        let this = unsafe { std::mem::transmute::<MrcS<T, Mutable>, MrcS<(), Mutable>>(value) };
        Self {
            inner: this,
            type_id: TypeId::of::<T>(),
        }
    }

    pub(crate) fn is<T: 'static + Sized>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    pub(crate) fn downcast<T: 'static + Sized>(self) -> Option<MrcS<T, Mutable>> {
        if self.is::<T>() {
            // SAFTY:
            // Transmute used for abstracting inner workings of Rc away
            // and to implement own version of dyn dispatch.
            let value = self.inner;
            Some(unsafe { std::mem::transmute::<MrcS<(), Mutable>, MrcS<T, Mutable>>(value) })
        } else {
            None
        }
    }
}

///
/// The default case of [MrcS] that allows internal mutability.
///
pub type Mrc<T> = MrcS<T, Mutable>;

///
/// A mutability state that **only** allows acces via [AsRef], [Deref]
/// and by extension [Borrow](std::borrow::Borrow).
///

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReadOnly;
impl MrcMutabilityState for ReadOnly {}

///
/// A mutability state that allows full access with [AsRef],
/// [Deref], [DerefMut] and by extension [Borrow](std::borrow::Borrow)
/// and [BorrowMut](std::borrow::BorrowMut).
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mutable;
impl MrcMutabilityState for Mutable {}

use private::MrcMutabilityState;
mod private {
    pub trait MrcMutabilityState {}
}

///
/// A version of [Rc] that allows internal mutation without explicit
/// syncroniszation (in single threaded enviroments).
///
/// # Safty contract
///
/// Since by default this type breaks rust's safty contract the caller must
/// ensure, that this type is only used in single threaded enviroments, thereby resolving
/// double-write or RWR problems. Addtionally no long living refernces to nested components
/// should be created, since holding a long-lived read reference to a datapoint that can be mutated
/// by a third party my invalidate the reference.
///
/// Note that these rules only apply to instances of `StatedMrc<T, Mutable>`. Should the
/// type state be set to `ReadOnly` the smart pointer cannot mutate the contained value.
///  
#[derive(Debug)]
pub struct MrcS<T, S>
where
    T: ?Sized,
    S: MrcMutabilityState,
{
    inner: Rc<UnsafeCell<T>>,
    phantom: PhantomData<S>,
}

impl<T, S> MrcS<T, S>
where
    S: MrcMutabilityState,
{
    ///
    /// Constructs a new [Mrc<T>].
    ///
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(UnsafeCell::new(value)),
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> MrcS<T, Mutable> {
    ///
    /// Declares a reference as read-only. Not reversable.
    ///
    pub fn make_readonly(self) -> MrcS<T, ReadOnly> {
        MrcS {
            inner: self.inner,
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> MrcS<T, ReadOnly> {
    ///
    /// Only for internal use
    ///
    #[allow(unused)]
    pub(crate) fn force_mutable(self) -> MrcS<T, Mutable> {
        MrcS {
            inner: self.inner,
            phantom: PhantomData,
        }
    }
}

impl<T, S> AsRef<T> for MrcS<T, S>
where
    T: ?Sized,
    S: MrcMutabilityState,
{
    fn as_ref(&self) -> &T {
        // SAFTY:
        // This deref in considered safe since it only extends Mrc
        // with the default Rc behaviour
        unsafe { &*self.inner.as_ref().get() }
    }
}

impl<T, S> Clone for MrcS<T, S>
where
    T: ?Sized,
    S: MrcMutabilityState,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            phantom: PhantomData,
        }
    }
}

impl<T, S> Deref for MrcS<T, S>
where
    T: ?Sized,
    S: MrcMutabilityState,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T> DerefMut for MrcS<T, Mutable>
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

impl<T, S, U> CoerceUnsized<MrcS<U, S>> for MrcS<T, S>
where
    T: ?Sized + Unsize<U>,
    S: MrcMutabilityState,
    U: ?Sized,
{
}

impl<T, S, U> DispatchFromDyn<MrcS<U, S>> for MrcS<T, S>
where
    T: ?Sized + Unsize<U>,
    S: MrcMutabilityState,
    U: ?Sized,
{
}

impl<T, S> PartialEq for MrcS<T, S>
where
    T: PartialEq,
    S: MrcMutabilityState,
{
    fn eq(&self, other: &Self) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T, S> Eq for MrcS<T, S>
where
    T: Eq,
    S: MrcMutabilityState,
{
}

impl<T, S> Hash for MrcS<T, S>
where
    T: Hash,
    S: MrcMutabilityState,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<T, S> PartialOrd for MrcS<T, S>
where
    T: PartialOrd,
    S: MrcMutabilityState,
{
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        self.deref().partial_cmp(rhs.deref())
    }
}

impl<T, S> Ord for MrcS<T, S>
where
    T: Ord,
    S: MrcMutabilityState,
{
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.deref().cmp(rhs.deref())
    }
}

pub(crate) struct SyncWrap<T> {
    inner: T,
}

impl<T> SyncWrap<T> {
    pub const fn new(item: T) -> Self {
        Self { inner: item }
    }
}

impl<T> Deref for SyncWrap<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for SyncWrap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// SAFTY:
// This wrapper should only be used to make statics thread safe,
// since by design event simulation is single-threded (in the same context).
unsafe impl<T> Sync for SyncWrap<T> {}
