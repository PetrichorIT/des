#![allow(unused)]
use std::{
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

pub(super) struct DLL<E> {
    head: Box<EventNode<E>>,
    tail: Box<EventNode<E>>,
    shared_len: Arc<AtomicUsize>,
}

struct EventNode<E> {
    value: Option<E>,
    time: Duration,

    handle: *mut *mut EventNode<E>,
    prev: *mut EventNode<E>,
    next: *mut EventNode<E>,
}

/// A handle to cancel an event.
#[derive(Debug)]
pub struct EventHandle<E> {
    inner: Box<*mut EventNode<E>>,
    shared_len: Arc<AtomicUsize>,
}

// IMPL: DLL

impl<T> DLL<T> {
    pub(super) fn new(len: Arc<AtomicUsize>) -> Self {
        let mut head = EventNode::empty(Duration::ZERO);
        let mut tail = EventNode::empty(Duration::MAX);

        let head_ptr: *mut EventNode<T> = &mut *head;
        let tail_ptr: *mut EventNode<T> = &mut *tail;

        head.next = tail_ptr;
        tail.prev = head_ptr;

        Self {
            head,
            tail,
            shared_len: len,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(super) fn len(&self) -> usize {
        self.iter().count()
    }

    /// Returns a ref valid,
    /// unless a handle is used.
    pub(super) fn front(&self) -> Option<(&T, Duration)> {
        // SAFTEY:
        // Value is guranteed to be valid since head->next is allways valid
        let front = unsafe { Box::from_raw(self.head.next) };
        if front.next.is_null() {
            std::mem::forget(front);
            None
        } else {
            // SAFTEY:
            // front is valid, and neither head nor tail so itt must contain a value
            let (v, t): (*const T, Duration) = (
                unsafe { front.value.as_ref().unwrap_unchecked() },
                front.time,
            );
            std::mem::forget(front);
            // SAFTEY:
            // Borrow points to valid datate an has a valid lifetime of (&self)
            // since the &self ref will gurantee that v is not dropped
            // unless via a handle
            Some((unsafe { &*v }, t))
        }
    }

    pub(super) fn add_to_tail(&mut self, value: T, time: Duration) -> EventHandle<T> {
        let (mut node, handle) = EventNode::new(value, time, self.shared_len.clone());
        let prev = self.tail.prev;
        node.prev = prev;

        // SAFTEY:
        // tail->prev allways points to a valid value (maybe a head)
        let mut prev = unsafe { Box::from_raw(prev) };
        prev.next = *handle.inner;
        std::mem::forget(prev);

        node.next = &mut *self.tail;
        self.tail.prev = *handle.inner;

        self.shared_len.fetch_add(1, SeqCst);
        std::mem::forget(node);
        handle
    }

    /// Inserts a new element into the queue, returing a Handle to
    /// cancel the event at will
    pub(super) fn add(&mut self, value: T, time: Duration) -> EventHandle<T> {
        let (mut node, handle) = EventNode::new(value, time, self.shared_len.clone());
        self.shared_len.fetch_add(1, SeqCst);

        // From back insert
        let mut cur: *mut EventNode<T> = &mut *self.tail;
        loop {
            // SAFTEY:
            // There a two cases
            // 1) cur is head -> since head has Duration::MIN the loop will
            //    break thus cur is a valid ptr.
            // 2) cur is not head (maybe tail) -> all such elements are guranteed to have
            //    valid prev ptrs.
            // Thus cur will be valid, non-null at the end of the loop.
            // This loop will terminated if there are no circles in the DLL
            unsafe {
                if (*cur).time > node.time {
                    cur = (*cur).prev;
                } else {
                    break;
                }
            }
        }

        // SAFTEY: cur is valid after the end of the loop (see aboth)
        let prev = cur;
        let next = unsafe { (*cur).next };

        node.prev = prev;
        node.next = next;

        // SAFTEY:
        // If the ptr is non-null it is valid,
        // since nodes are only dropped once they were removed from the DLL.
        // At removal, they remove ptrs to themselfs from other nodes.
        if !prev.is_null() {
            unsafe { (*prev).next = *handle.inner }
        }

        // SAFTEY: see prev
        if !next.is_null() {
            unsafe { (*next).prev = *handle.inner }
        }

        // Forget the node to leak the memory.
        std::mem::forget(node);
        handle
    }

    /// Removes the element with the earliest time from the queue.
    pub(super) fn pop_min(&mut self) -> Option<(T, Duration)> {
        let node = unsafe { Box::from_raw(self.head.next) };
        if node.next.is_null() {
            // The node that would have been returned is the tail.
            // Thus forgett this Box, since the tail is allready owned by self.
            std::mem::forget(node);
            None
        } else {
            self.shared_len.fetch_sub(1, SeqCst);

            // The node is not the tail (or the head),
            // Thus the node has valid ptrs to prev and next.
            // 1) This head.next will point to a valid node (may be tail)
            // 2) node.next will be a valid node
            // 3) node.next.prev will point ot a valid node (head)
            self.head.next = node.next;
            unsafe {
                (*node.next).prev = &mut *self.head;
            }

            // All references are removed from the DLL thus the node
            // is only owned by this instance.
            // Droping the node via into_inner is valid since the only remaining
            // ref (the NodeHandle) will be invalidated by this operation,
            // if nessecary
            Some(node.into_inner())
        }
    }

    pub(super) fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    pub(super) fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.into_iter()
    }
}

impl<T: Clone> Clone for DLL<T> {
    fn clone(&self) -> Self {
        let mut r = Self::new(self.shared_len.clone());
        for (event, time) in self.into_iter() {
            r.add(event.clone(), *time);
        }
        r
    }
}

impl<T> Debug for DLL<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head_ptr: *const EventNode<T> = &*self.head;
        let tail_ptr: *const EventNode<T> = &*self.tail;

        f.debug_struct("DLL")
            .field("head", &head_ptr)
            .field("tail", &tail_ptr)
            .finish()
    }
}

impl<T> Default for DLL<T> {
    fn default() -> Self {
        Self::new(Arc::new(AtomicUsize::new(0)))
    }
}

impl<T> Drop for DLL<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop_min() {}
    }
}

// EQ

impl<T: PartialEq> PartialEq for DLL<T> {
    fn eq(&self, other: &Self) -> bool {
        let mut lhs = self.iter();
        let mut rhs = other.iter();

        loop {
            let l = lhs.next();
            let r = rhs.next();
            if let Some(l) = l {
                if let Some(r) = r {
                    if l.0 != r.0 {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                if let Some(_) = r {
                    return false;
                } else {
                    break;
                }
            }
        }

        true
    }
}

impl<T: Eq> Eq for DLL<T> {}

// HASH

impl<T: Hash> Hash for DLL<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.iter().for_each(|v| v.hash(state))
    }
}

// FROM

impl<T> FromIterator<(T, Duration)> for DLL<T> {
    fn from_iter<I: IntoIterator<Item = (T, Duration)>>(iter: I) -> Self {
        let mut r = Self::new(Arc::new(AtomicUsize::new(0)));
        for (item, time) in iter {
            r.add(item, time);
        }
        r
    }
}

impl<T, const N: usize> From<[(T, Duration); N]> for DLL<T> {
    fn from(value: [(T, Duration); N]) -> Self {
        Self::from_iter(value)
    }
}

// IMPL: DLL Into Iter

pub struct Iter<'a, T> {
    marker: PhantomData<&'a DLL<T>>,
    cur: *mut EventNode<T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a T, &'a Duration);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFTEY:
        // Will point to a valid node since:
        // IA) head->next is a valid node
        // IS) each time the next node is check to be non-null (thus valid)
        let cur = unsafe { Box::from_raw(self.cur) };
        let result: Option<(*const T, *const Duration)> = {
            if cur.next.is_null() {
                // is tail
                None
            } else {
                self.cur = cur.next;
                // SAFTEY:
                // cur is allways valid + now non-tail
                Some((unsafe { cur.value.as_ref().unwrap_unchecked() }, &cur.time))
            }
        };
        std::mem::forget(cur);
        result.map(|(v, t)| unsafe { (&*v, &*t) })
    }
}

impl<'a, T> IntoIterator for &'a DLL<T> {
    type Item = (&'a T, &'a Duration);
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            marker: PhantomData,
            cur: self.head.next,
        }
    }
}

pub struct IterMut<'a, T> {
    marker: PhantomData<&'a mut DLL<T>>,
    cur: *mut EventNode<T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (&'a mut T, &'a Duration);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFTEY:
        // Will point to a valid node since:
        // IA) head->next is a valid node
        // IS) each time the next node is check to be non-null (thus valid)
        let mut cur = unsafe { Box::from_raw(self.cur) };
        let result: Option<(*mut T, *const Duration)> = {
            if cur.next.is_null() {
                // is tail
                None
            } else {
                self.cur = cur.next;
                // SAFTEY:
                // cur is allways valid + now non-tail
                Some((unsafe { cur.value.as_mut().unwrap_unchecked() }, &cur.time))
            }
        };
        std::mem::forget(cur);
        result.map(|(v, t)| unsafe { (&mut *v, &*t) })
    }
}

impl<'a, T> IntoIterator for &'a mut DLL<T> {
    type Item = (&'a mut T, &'a Duration);
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            marker: PhantomData,
            cur: self.head.next,
        }
    }
}

pub struct IntoIter<T> {
    dll: DLL<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = (T, Duration);
    fn next(&mut self) -> Option<Self::Item> {
        self.dll.pop_min()
    }
}

impl<T> IntoIterator for DLL<T> {
    type Item = (T, Duration);
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { dll: self }
    }
}

// IMPL: Node

impl<T> EventNode<T> {
    fn empty(time: Duration) -> Box<EventNode<T>> {
        Box::new(Self {
            value: None,
            time,
            handle: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
        })
    }

    fn new(
        value: T,
        time: Duration,
        shared_len: Arc<AtomicUsize>,
    ) -> (Box<EventNode<T>>, EventHandle<T>) {
        let mut node = Box::new(Self {
            value: Some(value),
            time,
            handle: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
        });

        let mut handle = EventHandle {
            inner: Box::new(&mut *node),
            shared_len,
        };

        node.handle = &mut *handle.inner;

        (node, handle)
    }

    fn into_inner(self) -> (T, Duration) {
        if !self.handle.is_null() {
            unsafe {
                (*self.handle) = std::ptr::null_mut();
            }
        }
        // SAFTEY:
        // This function may only be applied to nodes that are
        // neither head nor tail. Such notes allways contain a value
        (unsafe { self.value.unwrap_unchecked() }, self.time)
    }
}

impl<E> Debug for EventNode<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventNode")
            .field("handle", &self.handle)
            .field("prev", &self.prev)
            .field("next", &self.next)
            .field("time", &self.time)
            .field("value", &self.value.is_some())
            .finish()
    }
}

impl<T> EventHandle<T> {
    /// Cancels a event if the event is still in the Future-Event-Set
    ///
    /// This operation may only be performed if the Future-Event-Set
    /// is not currently accessed by another operation. Addditionally
    /// this operation may invalidate references provided
    /// by DLL::front.
    pub fn cancel(self) {
        if self.inner.is_null() {
            // HUH
        } else {
            let node = unsafe { Box::from_raw(*self.inner) };

            if !node.prev.is_null() {
                let mut prev = unsafe { Box::from_raw(node.prev) };
                prev.next = node.next;
                std::mem::forget(prev);
            }

            if !node.next.is_null() {
                let mut next = unsafe { Box::from_raw(node.next) };
                next.prev = node.prev;
                std::mem::forget(next);
            }

            self.shared_len.fetch_sub(1, SeqCst);
            drop(node);
        }
        std::mem::drop(self);
    }
}

impl<T> Drop for EventHandle<T> {
    fn drop(&mut self) {
        // remove handle entry from node
        if !self.inner.is_null() {
            unsafe {
                (**self.inner).handle = std::ptr::null_mut();
            }
        }
    }
}
