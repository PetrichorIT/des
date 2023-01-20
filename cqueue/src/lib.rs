#![feature(allocator_api)]
#![feature(nonnull_slice_from_raw_parts)]
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
use std::{collections::VecDeque, marker::PhantomData, ops::Rem, time::Duration};

// mod _alloc;
mod alloc;
mod linked_list;

pub(crate) use alloc::*;
use linked_list::DualLinkedList;

/// A calender queue.
///
/// This type acts as a sorter for entries of type E
/// that occure at a given point in time, represented by the
/// Duration type. This means that the fetch_next
/// method will allways return the entry with the smallest timestamp.
/// In general, this can be compared to a BinaryHeap where the entries
/// are a tupel (E, Duration) sorted by the Duration.
///
/// Note however that this datatype is optimized for use in a discrete
/// event simulation. Thus is supports O(1) inserts and removals, as
/// well as O(1) fetch_next. Note that this is a amorised analysis
/// assuming that the parameters are optimal for the given distribution
/// of event arrival times. Additionaly the CQueue does not allow for
/// the insertion of entries with a timestamp smaller that entries
/// that was last fetched (or Duration::ZERO initally).
///
#[derive(Debug)]
pub struct CQueue<E> {
    #[allow(unused)]
    pub(crate) alloc: Box<CQueueLLAllocatorInner>,

    // Parameters
    pub(crate) n: usize,
    pub(crate) t: Duration,
    pub(crate) t_nanos: u128,

    // Buckets
    pub(crate) zero_event_bucket: VecDeque<(E, Duration, usize)>,
    pub(crate) buckets: Vec<DualLinkedList<E>>,

    pub(crate) head: usize,

    pub(crate) t_current: Duration,
    pub(crate) t0: Duration,
    pub(crate) t1: Duration,
    pub(crate) t_all: u128,

    // Misc
    pub(crate) event_id: usize,
    pub(crate) len: usize,
}

/// A handle that identifies a event.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct EventHandle<E> {
    _phantom: PhantomData<E>,
    id: usize,
    time: Duration,
}

impl<E> CQueue<E> {
    /// Returns a String describing the datatype and its parameters.
    pub fn descriptor(&self) -> String {
        format!("CTimeVDeque({}, {:?})", self.n, self.t)
    }

    /// Returns the number of elements in the queue.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the number of element in the subset that is
    /// manage by the zero-event-time optimization.
    pub fn len_zero(&self) -> usize {
        self.zero_event_bucket.len()
    }

    /// Returns the number of elements in the subset that is
    /// not managed by the zero-event-time optimization.
    pub fn len_nonzero(&self) -> usize {
        self.len() - self.len_zero()
    }

    /// Indicates whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn metrics(&self) -> (usize, usize) {
        let (alloc, total) = self.alloc.metrics();
        let additional = std::mem::size_of::<Self>();
        let additional = additional + std::mem::size_of::<(E, Duration, usize)>() * self.len_zero();
        (alloc + additional, total + additional)
    }

    /// Returns the timestamp of the last emitted event.
    /// This acts as a lower bound to the insertion of new events.
    pub fn time(&self) -> Duration {
        self.t_current
    }

    /// Creates a new parameteriszed CQueue.
    pub fn new(n: usize, t: Duration) -> Self {
        // essentialy t*n
        let t_all = t.as_nanos() * n as u128;

        let alloc = Box::new(CQueueLLAllocatorInner::new());

        Self {
            n,
            t_nanos: t.as_nanos(),
            t,

            zero_event_bucket: VecDeque::with_capacity(64),
            buckets: std::iter::repeat_with(|| DualLinkedList::new(alloc.handle()))
                .take(n)
                .collect(),
            head: 0,
            t_current: Duration::ZERO,

            t0: Duration::ZERO,
            t1: t,

            t_all,

            alloc,
            event_id: 0,
            len: 0,
        }
    }

    ///
    /// Adds an event to the calenderqueue.
    ///
    /// Returns an event handle to cancel the event at will.
    ///
    /// # Panics
    ///
    /// This funtion panics if the timestamp violates the lower
    /// bound, defined by the timestamp of the last emitted event.
    ///
    pub fn add(&mut self, time: Duration, event: E) -> EventHandle<E> {
        assert!(
            time >= self.t_current,
            "Cannot add past event to calender queue"
        );

        self.len += 1;
        if time == self.t_current {
            let id = self.event_id;
            self.zero_event_bucket.push_back((event, time, id));
            self.event_id = id.wrapping_add(1);

            EventHandle {
                _phantom: PhantomData,
                id,
                time,
            }
        } else {
            // delta time ?

            let time_mod = time.as_nanos().rem(self.t_all);

            let index = time_mod / self.t_nanos;
            let index: usize = index as usize;
            let index = index % self.n;

            // find insert pos

            let id = self.event_id;
            self.buckets[index].add(event, time, id);
            self.event_id = id.wrapping_add(1);
            EventHandle {
                _phantom: PhantomData,
                id,
                time,
            }
        }
    }

    pub fn cancel(&mut self, handle: EventHandle<E>) {
        if handle.time >= self.t_current {
            if handle.time == self.t_current {
                if let Some((i, _)) = self
                    .zero_event_bucket
                    .iter()
                    .enumerate()
                    .find(|(_, v)| v.2 == handle.id)
                {
                    self.zero_event_bucket.remove(i);
                    self.len -= 1;
                }
            } else {
                let time_mod = handle.time.as_nanos().rem(self.t_all);

                let index = time_mod / self.t_nanos;
                let index: usize = index as usize;
                let index = index % self.n;

                if self.buckets[index].cancel(handle) {
                    self.len -= 1;
                }
            }
        }
    }

    ///
    /// Fetches the smalles event from the calender queue.
    ///
    /// # Panics
    ///
    /// This function assummes that the queue is not empty.
    /// If it is this function panics.
    ///
    pub fn fetch_next(&mut self) -> (E, Duration) {
        assert!(!self.is_empty(), "Cannot fetch from empty queue");

        if let Some((event, time, _)) = self.zero_event_bucket.pop_front() {
            self.len -= 1;
            return (event, time);
        }

        loop {
            // Move until full bucket is found.
            while self.buckets[self.head].is_empty() {
                self.head = (self.head + 1) % self.n;
                self.t0 += self.t;
                self.t1 += self.t;
            }

            // Bucket with > 0 elements found

            let min = self.buckets[self.head].front_time();
            if min > self.t1 {
                self.head = (self.head + 1) % self.n;
                self.t0 += self.t;
                self.t1 += self.t;
                continue;
            }

            self.t_current = min;

            // SAFTEY:
            // Bucket is non-empty, thus pop-min returns a valid value.
            self.len -= 1;
            return unsafe { self.buckets[self.head].pop_min().unwrap_unchecked() };
        }
    }
}

impl<E> Default for CQueue<E> {
    fn default() -> Self {
        Self::new(1024, Duration::from_millis(5))
    }
}

impl<E> Drop for CQueue<E> {
    fn drop(&mut self) {
        // Manually drop the DLL so that the alloc can be dropped last
        for dll in self.buckets.drain(..) {
            drop(dll)
        }
    }
}

#[cfg(test)]
mod tests;
