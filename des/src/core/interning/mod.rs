//!
//! A module for handeling efficent, dupliction free data storage.
//!
//! * This will only be visible when DES is build with the feature "pub-interning"
//!

use crate::util::mm::SyncCell;
use log::{trace, warn};
use std::alloc::{dealloc, Layout};
use std::any::{type_name, TypeId};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "net")]
use crate::net::Packet;

mod tests;

///
/// A manager for interned objects.
///
/// # Safty contract
///
/// Due to the design of the rust memory managment this module is inhereintly
/// unsafe. Nonetheless it can be compliant to the rust reference and drop invariants
/// if the user guarntees the following things:
///
/// - Reference to interned values will only be made using [InternedValue] and [TypedInternedValue].
/// - All types T are non-zero size.
/// - Boxed interning allways points to a valid Box that is not owned or referenced by somebody else.
///
/// By providing these invariants the Interner guarntees:
///
/// - That every reference ([InternedValue] or [TypedInternedValue]) is valid during its lifetime.
/// - That every interned value is dropped in place once all references ceese to exist.
/// - That every interned value is dropped with type specific drop code if the last reference
///     dropped is a [TypedInternedValue].
///
#[derive(Debug)]
pub struct Interner {
    contents: SyncCell<Vec<Option<InteredValueDescriptor>>>,
}

impl Interner {
    ///
    /// Creates a new empty interner.
    ///
    pub fn new() -> Self {
        Self {
            contents: SyncCell::new(Vec::new()),
        }
    }

    ///
    /// Interns a value T and retursn a [TypedInternedValue] referencing the interned value.
    ///
    #[allow(unused)]
    pub fn intern_typed<T>(&self, value: T) -> TypedInternedValue<'_, T>
    where
        T: 'static,
    {
        self.intern(value).cast()
    }

    ///
    /// Interns a value T.
    ///
    pub fn intern<T>(&self, value: T) -> InternedValue<'_>
    where
        T: 'static,
    {
        let boxed = Box::new(value);
        self.intern_boxed(boxed)
    }

    ///
    /// Interns an allready boxed value T and retursn a typed reference.
    ///
    #[allow(unused)]
    pub fn intern_boxed_typed<T>(&self, boxed: Box<T>) -> TypedInternedValue<'_, T>
    where
        T: 'static,
    {
        self.intern_boxed(boxed).cast()
    }

    ///
    /// Interns an allready boxed value T.
    ///
    pub fn intern_boxed<T>(&self, boxed: Box<T>) -> InternedValue<'_>
    where
        T: 'static,
    {
        assert!(
            std::mem::size_of::<T>() != 0,
            "Size of type T must not be zero."
        );

        let ptr = Box::into_raw(boxed) as *mut u8;

        let descriptor = InteredValueDescriptor {
            ref_count: 1,
            ty_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            ptr,
        };

        // # Safty
        // This operations is save since the safty contract of Intener
        // gurantees that & references allways point to valid memory.
        // Thereby if a cell is reused the no & references can exist pointing to the
        // cell. Following this concerning the cell only one &mut reference exists without
        // any & references.
        let contents = unsafe { &mut *self.contents.get() };

        for (index, item) in contents.iter_mut().enumerate() {
            if item.is_none() {
                // Use previous freed item.
                // println!(
                //     "[Interner] >> New #{} (filler) ty: {:?}",
                //     index, descriptor.ty_id
                // );
                *item = Some(descriptor);
                trace!(target: "interner", "Interning new {} value at ID: {}.", type_name::<T>(), index);

                return InternedValue {
                    interner: self,
                    index,
                };
            }
        }

        // Push new item
        let index = contents.len();
        trace!(target: "interner", "Interning new {} value at ID: {}.", type_name::<T>(), index);

        contents.push(Some(descriptor));

        InternedValue {
            interner: self,
            index,
        }
    }

    pub fn cast<T>(value: InternedValue) -> TypedInternedValue<T>
    where
        T: 'static,
    {
        trace!(target: "interner", "Casting to typed({}) ref for ID: {}", type_name::<T>(), value.index);
        let InternedValue { interner, index } = value;

        // # Safty
        // By the safty contract of Interner any Interned value must indirectly point to a valid
        // interned value, thereby the index points to a valid descriptor.
        let entry = unsafe { interner.get_mut(index) };

        assert_eq!(
            entry.ty_id,
            TypeId::of::<T>(),
            "Cannot cast value to invalid type '{}'",
            type_name::<T>()
        );

        // # Safty
        // By the safty contract the memory will be valid, and by checking the type id
        // the constructed reference will point to a valid instance of T,
        // so this is no UB.
        let reference = unsafe { &mut *(entry.ptr as *mut T) };

        // Since the ref is converted the ref count should not be
        // decreased by droppping 'value' nor increased by creating
        // the return type manually
        std::mem::forget(value);

        TypedInternedValue {
            interner,
            index,
            reference,
        }
    }

    pub fn uncast<T>(value: TypedInternedValue<T>) -> InternedValue
    where
        T: 'static,
    {
        trace!(target: "interner", "Uncasting typed({}) ref for ID: {}", type_name::<T>(), value.index);
        let TypedInternedValue {
            interner, index, ..
        } = value;

        std::mem::forget(value);

        InternedValue { interner, index }
    }

    pub fn drop_untyped(value: &mut InternedValue) {
        let InternedValue { interner, index } = *value;

        let remaining_ref = Self::dec_ref_count(interner, index);

        if remaining_ref == 0 {
            // Unsound drop, check usual suspects

            #[cfg(feature = "net")]
            if value.type_id() == TypeId::of::<Packet>() {
                return Self::drop_typed_raw::<Packet>(interner, index);
            }

            if value.type_id() == TypeId::of::<String>() {
                return Self::drop_typed_raw::<String>(interner, index);
            }
            if value.type_id() == TypeId::of::<Vec<u8>>() {
                return Self::drop_typed_raw::<Vec<u8>>(interner, index);
            }

            // Go no success
            warn!(target: "interner", "Dropping untyped value ID: {}", index);

            // # Safty
            // This is safe since all uses of get_mut() at internally and no
            // references leak.
            let contents = unsafe { &mut *interner.contents.get() };
            // # Safty
            // This is sound since the safty contract guarntees that 'ptr' points to
            // valid memory and 'layout' was derived at interning for Sized types.
            unsafe {
                dealloc(
                    contents[index].as_ref().unwrap().ptr,
                    contents[index].as_ref().unwrap().layout,
                )
            }

            contents[index] = None;
        }
    }

    pub fn drop_typed<T>(value: &mut TypedInternedValue<T>)
    where
        T: 'static,
    {
        let TypedInternedValue {
            interner, index, ..
        } = *value;

        let remaining_ref = Self::dec_ref_count(interner, index);
        if remaining_ref == 0 {
            Self::drop_typed_raw::<T>(interner, index);
        }
    }

    pub fn drop_typed_raw<T>(interner: &Interner, index: usize)
    where
        T: 'static,
    {
        trace!(target: "interner", "Dropping typed ({}) value ID: {}", type_name::<T>(), index);

        // # Safty
        // This is safe since all uses of get_mut() at internally and no
        // references leak.
        let contents = unsafe { &mut *interner.contents.get() };

        assert_eq!(
            contents[index].as_ref().unwrap().ref_count,
            0,
            "Cannot drop interned value with still valid references."
        );

        // # Safty
        // This is a safe operation since the refernced index is of type
        // T since this function is only called from a validated instance of
        // TypedInternedValue<T>.
        let boxed = unsafe { Box::from_raw(contents[index].as_ref().unwrap().ptr as *mut T) };
        drop(boxed);

        contents[index] = None;
    }

    /// Retrieves a entry at cell the given index.
    #[allow(unused_unsafe)]
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_mut(&self, index: usize) -> &mut InteredValueDescriptor {
        // # Safty
        // This is an internal fn that under the safty contract of
        // Interner is only used by IntenredValue<'a> instances.
        let contents = unsafe { &mut *self.contents.get() };

        let entry = contents
            .get_mut(index)
            .expect("Failed to resolve interned value. Index out of bounds.");
        entry
            .as_mut()
            .expect("Failed to resolve interned value. Value dropped.")
    }

    /// Registers a clone on the given value.
    fn inc_ref_count(&self, index: usize) {
        // # Safty
        // This operation is safe since [Self] is single threaded
        // and mutable referenced to ref_cell are only
        // temporary and not leaked outside Self.
        let entry = unsafe { self.get_mut(index) };
        trace!(target: "interner", "New ref for ID: {} up from {} to {}", index, entry.ref_count, entry.ref_count + 1);
        entry.ref_count += 1;
    }

    fn dec_ref_count(&self, index: usize) -> usize {
        // # Safty
        // This operation is safe since [Self] is single threaded
        // and mutable referenced to ref_cell are only
        // temporary and not leaked outside Self.
        let entry = unsafe { self.get_mut(index) };
        trace!(target: "interner", "Lost ref for ID: {} down from {} to {}", index, entry.ref_count, entry.ref_count - 1);

        entry.ref_count -= 1;
        entry.ref_count
    }

    fn type_id_of(&self, index: usize) -> TypeId {
        // # Safty
        // This operation is safe since [Self] is single threaded
        // and mutable referenced to ref_cell are only
        // temporary and not leaked outside Self.
        let entry = unsafe { self.get_mut(index) };
        entry.ty_id
    }

    /// Sanity check at the end of the simulation.
    pub fn fincheck(&self) {
        // # Safty
        // This is safe since all uses of get_mut() at internally and no
        // references leak.
        let contents = unsafe { &*self.contents.get() };
        for entry in contents.iter().flatten() {
            warn!(target: "interner", "Undisposed object after runtime end: {:?}", entry);
        }
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

// Only as sanity check
impl Drop for Interner {
    fn drop(&mut self) {
        self.fincheck();
    }
}

///
/// A descriptor for interned values like a untyped [Rc](std::rc::Rc).
///
#[derive(Debug, PartialEq, Eq)]
struct InteredValueDescriptor {
    ref_count: usize,
    ty_id: TypeId,
    layout: Layout,
    ptr: *mut u8,
}

///
/// A reference to a interned value of a given interner.
///
pub struct InternedValue<'a> {
    pub(crate) interner: &'a Interner,
    pub(crate) index: usize,
}

impl<'a> InternedValue<'a> {
    ///
    /// Casts self into a [TypedInternedValue] panicing of T doesn't match the T
    /// of the interned value.
    ///
    pub fn cast<T>(self) -> TypedInternedValue<'a, T>
    where
        T: 'static,
    {
        Interner::cast(self)
    }

    #[allow(unused)]
    pub fn can_cast<T>(&self) -> bool
    where
        T: 'static,
    {
        self.interner.type_id_of(self.index) == TypeId::of::<T>()
    }

    pub fn type_id(&self) -> TypeId {
        self.interner.type_id_of(self.index)
    }
}

impl Clone for InternedValue<'_> {
    fn clone(&self) -> Self {
        // Upon clone add 1 to the ref counter
        self.interner.inc_ref_count(self.index);

        Self {
            interner: self.interner,
            index: self.index,
        }
    }
}

impl Drop for InternedValue<'_> {
    fn drop(&mut self) {
        Interner::drop_untyped(self)
    }
}

impl Debug for InternedValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InternedValue {{ idx: {}, ... }}", self.index)
    }
}

///
/// A typed reference to a interned value.
///
pub struct TypedInternedValue<'a, T>
where
    T: 'static,
{
    interner: &'a Interner,
    index: usize,
    reference: &'a mut T,
}

impl<'a, T> TypedInternedValue<'a, T>
where
    T: 'static,
{
    ///
    /// Downgards self to a [InternedValue] losing type information in the process.
    ///
    /// # Note
    ///
    /// This should only be done if the procuded value is not the last reference to the interned value,
    /// the procuded value will be upgraded later reliably.
    /// If that would be the case, the interned value would be dropped without type information
    /// leading to a potentially incomplete drop.
    ///
    #[allow(unused)]
    pub fn uncast(self) -> InternedValue<'a> {
        Interner::uncast(self)
    }

    #[allow(unused)]
    pub fn type_id(&self) -> TypeId {
        self.interner.type_id_of(self.index)
    }
}

impl<T> Deref for TypedInternedValue<'_, T>
where
    T: 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.reference
    }
}

impl<T> DerefMut for TypedInternedValue<'_, T>
where
    T: 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reference
    }
}

// TODO:
// Remove this trait impl since users expect this to clone the
// inner value, resulting in uncontrolled mutation from multiple
// non depdendent sources.
//
// Maybe impl 'Clone' as true clone and make ref-clones internal.
impl<'a, T> Clone for TypedInternedValue<'a, T>
where
    T: 'static,
{
    fn clone(&self) -> Self {
        // Upon clone add 1 to the ref counter
        self.interner.inc_ref_count(self.index);

        let raw_interned = InternedValue {
            interner: self.interner,
            index: self.index,
        };

        // This vodo is nessecary to fight of the
        // lifetimes system
        raw_interned.cast()
    }
}

impl<T> Drop for TypedInternedValue<'_, T>
where
    T: 'static,
{
    fn drop(&mut self) {
        Interner::drop_typed(self)
    }
}
