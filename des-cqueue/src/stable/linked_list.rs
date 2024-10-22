use super::{alloc::CQueueLLAllocator, boxed::LocalBox, EventHandle};
use std::{fmt::Debug, time::Duration};

pub(crate) struct DualLinkedList<E> {
    alloc: CQueueLLAllocator,
    head: LocalBox<EventNode<E>>,
    tail: LocalBox<EventNode<E>>,
    len: usize,
}

#[derive(Debug, Clone)]
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

    pub(super) fn cancel(&mut self, handle: &EventHandle<T>) -> bool {
        let mut cur = self.head.next;
        unsafe {
            while !(*cur).next.is_null() {
                if (*cur).id == handle.id {
                    // remove
                    let mut cur = LocalBox::from_raw_in(cur, self.alloc);
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
        let ptr = self.head.next;
        let front = unsafe { &mut *ptr };
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
        let mut node = unsafe { LocalBox::from_raw_in(self.head.next, self.alloc) };
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
            Some(EventNode::into_inner(node))
        }
    }
}

impl<T> Drop for DualLinkedList<T> {
    fn drop(&mut self) {
        while self.pop_min().is_some() {}
    }
}

// EQ

// HASH

// IMPL: DLL Into Iter

// IMPL: Node

impl<T> EventNode<T> {
    pub(super) fn empty(time: Duration, alloc: CQueueLLAllocator) -> LocalBox<EventNode<T>> {
        LocalBox::new_in(
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
    ) -> LocalBox<EventNode<T>> {
        LocalBox::new_in(
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
    fn into_inner(mut this: LocalBox<Self>) -> (T, Duration) {
        // SAFTEY:
        // This function may only be applied to nodes that are
        // neither head nor tail. Such notes allways contain a value
        (unsafe { this.value.take().unwrap_unchecked() }, this.time)
    }
}
