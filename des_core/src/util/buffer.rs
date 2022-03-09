#[cfg(feature = "net-static")]
use log::info;
#[cfg(feature = "net-static")]
use std::any::type_name;

use std::hash::Hash;
use std::ops::Deref;
use std::slice::{Iter, IterMut};

///
/// A buffer for self-indexable objects, enabling static
/// optimization.
///
/// # Feature "static"
///
/// If this feature is active this buffer wil be optimized to allow O(1)
/// accesses at runtime after the buffer is locked. Note that locking of the buffer
/// prevents the removal or insertion of new elements into the buffer.
/// Insertion can also be optimized to O(1) (ignoring grows) if and only
/// if elements are inserted in the correct order.
///
/// # Default behaviour
///
/// If no static optimization is performed, both insert and access are O(log n)
/// with caching optimizations for [IdBufferRef]. Insertions and removals are
/// allowed at any point in the process.
///
pub struct IdBuffer<T>
where
    T: Indexable,
{
    inner: Vec<T>,

    #[cfg(not(feature = "net-static"))]
    gen: usize,

    #[cfg(feature = "net-static")]
    locked: bool,
}

impl<T> IdBuffer<T>
where
    T: Indexable,
{
    ///
    /// Creates a new empty buffer.
    ///
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    ///
    /// Creates a new buffer that does not need reallocation until
    /// greater than 'cap' elements are inserted.
    ///
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Vec::with_capacity(cap),

            #[cfg(not(feature = "net-static"))]
            gen: 0,

            #[cfg(feature = "net-static")]
            locked: false,
        }
    }

    ///
    /// Inserts an element into the buffer, sorting it at the best allocated
    /// stop. Returns a reference to the element that should only be used very temporary.
    ///
    /// # Complexity
    ///
    /// O(1) with feature "static" and in-order insertions.
    /// O(log n) elsewhere.
    ///
    pub fn insert(&mut self, item: T) -> &mut T {
        #[cfg(feature = "net-static")]
        assert!(!self.locked, "Cannot insert element into locked buffer");

        // Shortcut to speed up static in-line inserts.
        // Usually in static cases insertions are allready in order but this leads
        // to worst case prefomace of 'binary_serach_by_key'.
        // so check this shortcut.
        #[cfg(feature = "net-static")]
        match self.inner.last() {
            Some(element) => {
                if element.id() < item.id() {
                    self.inner.push(item);
                    return self.inner.last_mut().unwrap();
                }
            }
            None => {
                self.inner.push(item);
                return self.inner.last_mut().unwrap();
            }
        }

        let id = item.id();
        let insert_at = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(insert_at) | Err(insert_at) => insert_at,
        };

        self.inner.insert(insert_at, item);

        #[cfg(not(feature = "net-static"))]
        {
            self.gen += 1;
        }

        &mut self.inner[insert_at]
    }

    ///
    /// Removes an element from the buffer, returning whether the element
    /// was found and removed.
    ///
    #[cfg(not(feature = "net-static"))]
    pub fn remove(&mut self, id: T::Id) -> bool {
        let idx = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(idx) => idx,
            Err(_) => return false,
        };

        self.inner.remove(idx);
        true
    }

    ///
    /// Locks the buffer checking the validity of the indices,
    /// forbidding the insertion of any future elements.
    /// This ensures that the memory will not be unmapped allowing
    /// direct ptr optiominzation.
    ///
    #[cfg(feature = "net-static")]
    pub fn lock(&mut self) {
        info!(
            target: &format!("Buffer<{}>", type_name::<T>()),
            "Locked with {} elements",
            self.inner.len()
        );

        // DEBUG ONLY
        println!(
            "Locked Buffer<{}> with {} elements",
            type_name::<T>(),
            self.inner.len()
        );

        self.locked = true;
        for i in 0..self.inner.len() {
            assert_eq!(i, self.inner[i].id().as_index())
        }
    }

    ///
    /// Returns a reference to the entire content buffer.
    ///
    pub fn contents(&self) -> &Vec<T> {
        &self.inner
    }

    ///
    /// Returns a mutable reference to the entire content buffer.
    ///
    pub fn contents_mut(&mut self) -> &mut Vec<T> {
        &mut self.inner
    }

    ///
    /// Retrieves a element read-only by using its id.
    ///
    /// Uses static indexing if buffer is locked and feature = "static"
    /// is activated. Else uses binary search.
    ///
    /// # Complexity
    ///
    /// O(1) or O(log n)
    ///
    pub fn get(&self, id: T::Id) -> Option<&T> {
        #[cfg(feature = "net-static")]
        if self.locked {
            return Some(&self.inner[id.as_index()]);
        }

        let idx = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(idx) => idx,
            Err(_) => return None,
        };

        Some(&self.inner[idx])
    }

    ///
    /// Retrieves a element mutably by using its id.
    ///
    /// Uses static indexing if buffer is locked and feature = "net-static"
    /// is activated. Else uses binary search.
    ///
    /// # Complexity
    ///
    /// O(1) or O(log n)
    ///
    pub fn get_mut(&mut self, id: T::Id) -> Option<&mut T> {
        #[cfg(feature = "net-static")]
        if self.locked {
            return Some(&mut self.inner[id.as_index()]);
        }

        let idx = match self.inner.binary_search_by_key(&id, |c| c.id()) {
            Ok(idx) => idx,
            Err(_) => return None,
        };

        Some(&mut self.inner[idx])
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.inner.iter_mut()
    }
}

impl<T> Default for IdBuffer<T>
where
    T: Indexable,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "net-static"))]
use crate::util::SyncCell;

///
/// A semi-owned reference to a object stored in the buffer.
/// There should only exist one [IdBufferRef] per buffered object,
/// but it is possible to create multipled ones.
///
#[derive(Debug)]
pub struct IdBufferRef<T>
where
    T: Indexable,
{
    buffer: *mut IdBuffer<T>,
    id: T::Id,

    #[cfg(not(feature = "net-static"))]
    direct_ptr: SyncCell<Option<DirectPtr<T>>>,
}

impl<T> IdBufferRef<T>
where
    T: Indexable,
{
    pub fn id(&self) -> T::Id {
        self.id
    }

    ///
    /// Creates a new strong ref to a object referenced by and id
    /// in the given buffer.
    ///
    pub fn new(id: T::Id, buffer: &mut IdBuffer<T>) -> Self {
        Self {
            id,
            buffer,

            #[cfg(not(feature = "net-static"))]
            direct_ptr: SyncCell::new(None),
        }
    }

    #[cfg(not(feature = "net-static"))]
    #[allow(clippy::mut_from_ref)]
    fn direct(&self) -> &mut Option<DirectPtr<T>> {
        unsafe { &mut *self.direct_ptr.get() }
    }

    ///
    /// Resolves the ref to a read-only intrincis reference.
    ///
    pub fn get(&self) -> &T {
        //
        // # Safty
        //
        // This is safe since those functions will only be called as long
        // as the simulation is running, which implies that the NetworkRuntime
        // is still alive thereby its member 'gate_buffer' as well. Since GateBuffer
        // and NetworkRuntime are Sized there will be no reallocs.
        //
        let buffer = unsafe { &*self.buffer };

        //
        // Direct links will only be used when no implicite O(1) indexing is possible.
        //
        #[cfg(not(feature = "net-static"))]
        let buffer_gen = buffer.gen;

        //
        // # Safty
        //
        // This is safe since the ptr was created from a valid instance
        // in a previous call of this fn, and the refernced buffer has not changed
        // as indicated by gen.
        //
        // Direct links will only be used when no implicite O(1) indexing is possible.
        //
        #[cfg(not(feature = "net-static"))]
        if let Some(DirectPtr { gen, ptr }) = self.direct() {
            if *gen == buffer_gen {
                return unsafe { &**ptr };
            }
        }

        let obj = buffer.get(self.id).unwrap();

        //
        // # Safty
        //
        // This is save since only self may mutate gen and this simulation is single-threaded.
        //
        // Direct links will only be used when no implicite O(1) indexing is possible.
        //
        #[cfg(not(feature = "net-static"))]
        {
            let r = self.direct();
            *r = Some(DirectPtr {
                gen: buffer_gen,
                ptr: (obj as *const T) as *mut T,
            });
        }

        obj
    }

    ///
    /// Resolves the ref to a mutable intrincis reference.
    ///
    pub fn get_mut(&mut self) -> &mut T {
        //
        // # Safty
        //
        // This is safe since those functions will only be called as long
        // as the simulation is running, which implies that the NetworkRuntime
        // is still alive thereby its member 'gate_buffer' as well. Since GateBuffer
        // and NetworkRuntime are Sized there will be no reallocs.
        //
        let buffer = unsafe { &mut *self.buffer };

        //
        // Direct links will only be used when no implicite O(1) indexing is possible.
        //
        #[cfg(not(feature = "net-static"))]
        let buffer_gen = buffer.gen;

        //
        // # Safty
        //
        // This is safe since the ptr was created from a valid instance
        // in a previous call of this fn, and the refernced buffer has not changed
        // as indicated by gen.
        //
        // Direct links will only be used when no implicite O(1) indexing is possible.
        //
        #[cfg(not(feature = "net-static"))]
        if let Some(DirectPtr { gen, ptr }) = self.direct() {
            if *gen == buffer_gen {
                return unsafe { &mut **ptr };
            }
        }

        let obj = buffer.get_mut(self.id).unwrap();

        //
        // # Safty
        //
        // This is save since only self may mutate gen and this simulation is single-threaded.
        //
        // Direct links will only be used when no implicite O(1) indexing is possible.
        //
        #[cfg(not(feature = "net-static"))]
        {
            let r = self.direct();
            *r = Some(DirectPtr {
                gen: buffer_gen,
                ptr: obj,
            });
        }

        obj
    }
}

impl<T> Clone for IdBufferRef<T>
where
    T: Indexable,
{
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer,
            id: self.id,
        }
    }
}

impl<T> PartialEq for IdBufferRef<T>
where
    T: Indexable,
{
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for IdBufferRef<T> where T: Indexable {}

impl<T> Hash for IdBufferRef<T>
where
    T: Indexable,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.buffer.hash(state);
        self.id.as_usize().hash(state)
    }
}

#[cfg(not(feature = "net-static"))]
#[derive(Debug, Clone)]
struct DirectPtr<T> {
    gen: usize,
    ptr: *mut T,
}

///
/// A type that has a id that can be used as a index.
///
pub trait Indexable {
    /// The type of IDs used to index the type.
    type Id: IdAsIndex;

    ///
    /// Returns the identifer of this instance.
    ///
    fn id(&self) -> Self::Id;
}

impl<T, S> Indexable for T
where
    T: Deref<Target = S>,
    S: ?Sized + Indexable,
{
    type Id = S::Id;

    fn id(&self) -> Self::Id {
        self.deref().id()
    }
}

///
/// A id that can be used as a index.
///
pub trait IdAsIndex: Copy + Ord {
    ///
    /// The first valid id in the runtime.
    ///
    const MIN: Self;

    ///
    /// Returns the raw primitiv the UID is contructed over.
    ///
    fn as_usize(&self) -> usize;

    ///
    /// Returns the ID normalized as a index based on the usize
    /// value and the MIN.
    ///
    fn as_index(&self) -> usize
    where
        Self: Sized,
    {
        self.as_usize() - Self::MIN.as_usize()
    }
}
