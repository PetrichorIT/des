use std::any::TypeId;
use std::ops::{Deref, DerefMut};
use utils::SyncCell;

#[derive(Debug)]
pub struct Interner {
    contents: SyncCell<Vec<Option<InteredValueDescriptor>>>,
}

impl Interner {
    pub fn new() -> Self {
        Self {
            contents: SyncCell::new(Vec::new()),
        }
    }

    pub fn intern_typed<T: 'static>(&self, value: T) -> TypedInternedValue<'_, T> {
        self.intern(value).cast()
    }

    pub fn intern<T: 'static>(&self, value: T) -> InternedValue<'_> {
        let boxed = Box::new(value);
        self.intern_boxed(boxed)
    }

    pub fn intern_boxed_typed<T: 'static>(&self, boxed: Box<T>) -> TypedInternedValue<'_, T> {
        self.intern_boxed(boxed).cast()
    }

    pub fn intern_boxed<T: 'static>(&self, boxed: Box<T>) -> InternedValue<'_> {
        let ptr = Box::into_raw(boxed) as *mut ();

        let descriptor = InteredValueDescriptor {
            ref_count: 1,
            ty_id: TypeId::of::<T>(),
            ptr,
        };

        // # Safty
        // This operations is save since the safty contract of Intener
        // gurantees that & references allways point to valid memory.
        // Thereby if a cell is reused the no & references can exist pointing to the
        // cell. Following this concerning the cell only one &mut reference exists without
        // any & references.
        let contents = unsafe { &mut *self.contents.get() };

        for index in 0..contents.len() {
            if contents[index].is_none() {
                // Use previous freed item.
                // println!(
                //     "[Interner] >> New #{} (filler) ty: {:?}",
                //     index, descriptor.ty_id
                // );
                contents[index] = Some(descriptor);

                return InternedValue {
                    interner: self,
                    index,
                };
            }
        }

        // Push new item
        let index = contents.len();
        // println!(
        //     "[Interner] >> New #{} (pusher) ty: {:?}",
        //     index, descriptor.ty_id
        // );

        contents.push(Some(descriptor));

        InternedValue {
            interner: self,
            index,
        }
    }

    #[allow(unused_unsafe)]
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

    fn clone_interned(&self, index: usize) {
        // # Safty
        // This operation is safe since [Self] is single threaded
        // and mutable referenced to ref_cell are only
        // temporary and not leaked outside Self.
        let entry = unsafe { self.get_mut(index) };

        // println!(
        //     "[Interner] >> Clone #{} {} --> {}",
        //     index,
        //     entry.ref_count,
        //     entry.ref_count + 1
        // );

        entry.ref_count += 1;
    }

    /// Checks reference counts and returns whether the value should be dropped.
    fn predrop_interned(&self, index: usize) -> bool {
        // # Safty
        // This operation is safe since [Self] is single threaded
        // and mutable referenced to ref_cell are only
        // temporary and not leaked outside Self.
        let entry = unsafe { self.get_mut(index) };

        // println!(
        //     "[Interner] >> Predrop #{} {} --> {}",
        //     index,
        //     entry.ref_count,
        //     entry.ref_count.saturating_sub(1)
        // );

        entry.ref_count -= 1;

        entry.ref_count == 0
    }

    fn drop_untyped_interned(&self, index: usize) {
        // println!("[Interner] >> Drop (untyped) #{}", index);

        let contents = unsafe { &mut *self.contents.get() };
        contents[index] = None;
    }

    fn drop_typed_interned<T: 'static>(&self, index: usize) {
        let contents = unsafe { &mut *self.contents.get() };
        assert_eq!(
            contents[index].as_ref().unwrap().ref_count,
            0,
            "Cannot drop interned value with still valid references."
        );

        // println!("[Interner] >> Drop (typed) #{}", index);

        // # Safty
        // This is a safe operation since the refernced index is of type
        // T since this function is only called from a validated instance of
        // TypedInternedValue<T>.
        let boxed = unsafe { Box::from_raw(contents[index].as_ref().unwrap().ptr as *mut T) };
        drop(boxed);

        contents[index] = None;
    }

    pub fn fincheck(&self) {
        let contents = unsafe { &*self.contents.get() };
        for entry in contents {
            if let Some(entry) = entry {
                eprintln!("[ERROR] Undisposed obj after runtime end: {:?}", entry);
            }
        }
    }
}

// Only as sanity check
impl Drop for Interner {
    fn drop(&mut self) {
        let contents = unsafe { &*self.contents.get() };
        for entry in contents {
            if let Some(entry) = entry {
                eprintln!("[ERROR] Undisposed obj after runtime end: {:?}", entry);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct InteredValueDescriptor {
    ref_count: usize,
    ty_id: TypeId,
    ptr: *mut (),
}

pub struct InternedValue<'a> {
    interner: &'a Interner,
    index: usize,
}

impl<'a> InternedValue<'a> {
    pub fn cast<T: 'static>(self) -> TypedInternedValue<'a, T> {
        // # Safty
        // By the safty contract of Interner any Interned value must indirectly point to a valid
        // interned value, thereby the index points to a valid descriptor.
        let entry = unsafe { self.interner.get_mut(self.index) };
        assert_eq!(
            entry.ty_id,
            TypeId::of::<T>(),
            "Cannot cast value to invalid type T"
        );

        // # Safty
        // By the safty contract the memory will be valid, and by checking the type id
        // the constructed reference will point to a valid instance of T,
        // so this is no UB.
        let reference = unsafe { &mut *(entry.ptr as *mut T) };

        // Since self gets dropped either way TypedInternedValue is a new
        // reference to the interned value.
        self.interner.clone_interned(self.index);

        TypedInternedValue {
            interner: self.interner,
            index: self.index,
            reference,
        }
    }
}

impl Clone for InternedValue<'_> {
    fn clone(&self) -> Self {
        // Upon clone add 1 to the ref counter
        self.interner.clone_interned(self.index);

        Self {
            interner: self.interner,
            index: self.index,
        }
    }
}

impl Drop for InternedValue<'_> {
    fn drop(&mut self) {
        // If a ref is dropped sub 1 from the ref counter
        let final_drop = self.interner.predrop_interned(self.index);

        // If final drop, use provided typeinfo for sound drop
        if final_drop {
            self.interner.drop_untyped_interned(self.index)
        }
    }
}

pub struct TypedInternedValue<'a, T: 'static> {
    interner: &'a Interner,
    index: usize,
    reference: &'a mut T,
}

impl<'a, T> TypedInternedValue<'a, T> {
    pub(crate) fn uncast(self) -> InternedValue<'a> {
        // Since self is still droped register downcasted value as clone
        self.interner.clone_interned(self.index);

        InternedValue {
            interner: self.interner,
            index: self.index,
        }
    }
}

impl<T: 'static> Deref for TypedInternedValue<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.reference
    }
}

impl<T: 'static> DerefMut for TypedInternedValue<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reference
    }
}

impl<'a, T: 'static> Clone for TypedInternedValue<'a, T> {
    fn clone(&self) -> Self {
        // Upon clone add 1 to the ref counter
        self.interner.clone_interned(self.index);

        let raw_interned = InternedValue {
            interner: self.interner,
            index: self.index,
        };

        raw_interned.cast()
    }
}

impl<T: 'static> Drop for TypedInternedValue<'_, T> {
    fn drop(&mut self) {
        // If a ref is dropped sub 1 from the ref counter
        let final_drop = self.interner.predrop_interned(self.index);

        // If final drop, use provided typeinfo for sound drop
        if final_drop {
            self.interner.drop_typed_interned::<T>(self.index)
        }
    }
}