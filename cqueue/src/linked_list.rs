use crate::{alloc::CQueueLLAllocator, EventHandle};
use std::{fmt::Debug, hash::Hash, marker::PhantomData, time::Duration};

pub(super) struct DualLinkedList<E> {
    alloc: CQueueLLAllocator,
    head: Box<EventNode<E>, CQueueLLAllocator>,
    tail: Box<EventNode<E>, CQueueLLAllocator>,
    len: usize,
}

#[derive(Clone)]
pub struct EventNode<E> {
    pub(super) value: Option<E>,
    pub(super) time: Duration,

    pub(super) id: usize,

    pub(super) prev: *mut EventNode<E>,
    pub(super) next: *mut EventNode<E>,
}

// IMPL: DLL

impl<T> DualLinkedList<T> {
    pub(super) fn new(alloc: CQueueLLAllocator) -> Self {
        let mut head = EventNode::empty(Duration::ZERO, alloc);
        let mut tail = EventNode::empty(Duration::MAX, alloc);

        let head_ptr: *mut EventNode<T> = &mut *head;
        let tail_ptr: *mut EventNode<T> = &mut *tail;

        head.next = tail_ptr;
        tail.prev = head_ptr;

        Self {
            alloc,
            head,
            tail,
            len: 0,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn cancel(&mut self, handle: EventHandle<T>) -> bool {
        let mut cur = self.head.next;
        unsafe {
            while !(*cur).next.is_null() {
                if (*cur).id == handle.id {
                    // remove
                    let cur = Box::from_raw_in(cur, self.alloc);
                    (*cur.prev).next = cur.next;
                    (*cur.next).prev = cur.prev;
                    self.len -= 1;

                    drop(cur);
                    return true;
                }
                cur = (*cur).next;
            }
        }
        false
    }

    pub(super) fn front_time(&self) -> Duration {
        // SAFTEY:
        // Value is guranteed to be valid since head->next is allways valid
        let front = unsafe { &mut *self.head.next };
        if front.next.is_null() {
            Duration::MAX
        } else {
            // SAFTEY:
            // front is valid, and neither head nor tail so itt must contain a value
            front.time
        }
    }

    /// Inserts a new element into the queue, returing a Handle to
    /// cancel the event at will
    pub(super) fn add(&mut self, event: T, time: Duration, event_id: usize) {
        let mut node = EventNode::new(event, time, event_id, self.alloc);
        self.len += 1;
        let node_ptr: *mut EventNode<T> = &mut *node;

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
            unsafe { (*prev).next = node_ptr }
        }

        // SAFTEY: see prev
        if !next.is_null() {
            unsafe { (*next).prev = node_ptr }
        }

        // Forget the node to leak the memory.
        std::mem::forget(node);
    }

    /// Removes the element with the earliest time from the queue.
    pub(super) fn pop_min(&mut self) -> Option<(T, Duration)> {
        let node = unsafe { Box::from_raw_in(self.head.next, self.alloc) };
        if node.next.is_null() {
            // The node that would have been returned is the tail.
            // Thus forgett this Box, since the tail is allready owned by self.
            std::mem::forget(node);
            None
        } else {
            self.len -= 1;

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
            let v = node.into_inner();
            Some(v)
        }
    }

    pub(super) fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    #[allow(unused)]
    pub(super) fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.into_iter()
    }
}

// impl<T: Clone> Clone for DLL<T> {
//     fn clone(&self) -> Self {
//         let mut r = Self::new(self.shared_len.clone(), todo!());
//         for (event, time) in self.into_iter() {
//             r.add(EventNode::new(event.clone(), *time, r.shared_len.clone()).0);
//         }
//         r
//     }
// }

impl<T> Debug for DualLinkedList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let head_ptr: *const EventNode<T> = &*self.head;
        let tail_ptr: *const EventNode<T> = &*self.tail;

        f.debug_struct("DLL")
            .field("head", &head_ptr)
            .field("tail", &tail_ptr)
            .finish()
    }
}

// impl<T> Default for DLL<T> {
//     fn default() -> Self {
//         Self::new(Arc::new(AtomicUsize::new(0)), todo!())
//     }
// // }

impl<T> Drop for DualLinkedList<T> {
    fn drop(&mut self) {
        while self.pop_min().is_some() {}
    }
}

// EQ

impl<T: PartialEq> PartialEq for DualLinkedList<T> {
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
            } else if r.is_some() {
                return false;
            } else {
                break;
            }
        }

        true
    }
}

impl<T: Eq> Eq for DualLinkedList<T> {}

// HASH

impl<T: Hash> Hash for DualLinkedList<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.iter().for_each(|v| v.hash(state))
    }
}

// FROM

// impl<T> FromIterator<(T, Duration)> for DLL<T> {
//     fn from_iter<I: IntoIterator<Item = (T, Duration)>>(iter: I) -> Self {
//         let mut r = Self::new(todo!());
//         for (item, time) in iter {
//             r.add(EventNode::new(item, time, r.shared_len.clone()).0);
//         }
//         r
//     }
// }

// impl<T, const N: usize> From<[(T, Duration); N]> for DLL<T> {
//     fn from(value: [(T, Duration); N]) -> Self {
//         Self::from_iter(value)
//     }
// }

// IMPL: DLL Into Iter

pub struct Iter<'a, T> {
    marker: PhantomData<&'a DualLinkedList<T>>,
    cur: *mut EventNode<T>,
    alloc: CQueueLLAllocator,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a T, &'a Duration);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFTEY:
        // Will point to a valid node since:
        // IA) head->next is a valid node
        // IS) each time the next node is check to be non-null (thus valid)
        let cur = unsafe { Box::from_raw_in(self.cur, self.alloc) };
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

impl<'a, T> IntoIterator for &'a DualLinkedList<T> {
    type Item = (&'a T, &'a Duration);
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            marker: PhantomData,
            cur: self.head.next,
            alloc: self.alloc,
        }
    }
}

pub struct IterMut<'a, T> {
    marker: PhantomData<&'a mut DualLinkedList<T>>,
    cur: *mut EventNode<T>,
    alloc: CQueueLLAllocator,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (&'a mut T, &'a Duration);

    fn next(&mut self) -> Option<Self::Item> {
        // SAFTEY:
        // Will point to a valid node since:
        // IA) head->next is a valid node
        // IS) each time the next node is check to be non-null (thus valid)
        let mut cur = unsafe { Box::from_raw_in(self.cur, self.alloc) };
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

impl<'a, T> IntoIterator for &'a mut DualLinkedList<T> {
    type Item = (&'a mut T, &'a Duration);
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            marker: PhantomData,
            cur: self.head.next,
            alloc: self.alloc,
        }
    }
}

pub struct IntoIter<T> {
    dll: DualLinkedList<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = (T, Duration);
    fn next(&mut self) -> Option<Self::Item> {
        self.dll.pop_min()
    }
}

impl<T> IntoIterator for DualLinkedList<T> {
    type Item = (T, Duration);
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { dll: self }
    }
}

// IMPL: Node

impl<T> EventNode<T> {
    pub(super) fn empty(
        time: Duration,
        alloc: CQueueLLAllocator,
    ) -> Box<EventNode<T>, CQueueLLAllocator> {
        Box::new_in(
            Self {
                value: None,
                id: 0,
                time,
                prev: std::ptr::null_mut(),
                next: std::ptr::null_mut(),
            },
            alloc,
        )
    }

    pub(super) fn new(
        value: T,
        time: Duration,
        id: usize,
        alloc: CQueueLLAllocator,
    ) -> Box<EventNode<T>, CQueueLLAllocator> {
        Box::new_in(
            Self {
                value: Some(value),
                time,
                id,
                prev: std::ptr::null_mut(),
                next: std::ptr::null_mut(),
            },
            alloc,
        )
    }

    #[allow(clippy::boxed_local)]
    fn into_inner(mut self: Box<Self, CQueueLLAllocator>) -> (T, Duration) {
        // SAFTEY:
        // This function may only be applied to nodes that are
        // neither head nor tail. Such notes allways contain a value
        (unsafe { self.value.take().unwrap_unchecked() }, self.time)
    }
}

impl<E> Debug for EventNode<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventNode")
            .field("prev", &self.prev)
            .field("next", &self.next)
            .field("time", &self.time)
            .field("value", &self.value.is_some())
            .finish()
    }
}
