mod linked_list;

use linked_list::*;
use std::{
    ops::Rem,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
    time::Duration,
};

pub use linked_list::EventHandle;

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
#[derive(Debug, Clone)]
pub struct CQueue<E> {
    // Parameters
    pub(crate) n: usize,
    pub(crate) t: Duration,

    pub(crate) t_nanos: u128,

    // Buckets
    pub(crate) zero_event_bucket: DLL<E>,
    pub(crate) buckets: Vec<DLL<E>>,
    pub(crate) head: usize,

    pub(crate) t_current: Duration,

    pub(crate) t0: Duration,
    pub(crate) t1: Duration,
    pub(crate) t_all: u128,

    // Misc
    pub(crate) empty_nodes: Vec<Box<EventNode<E>>>,
    pub(crate) len: Arc<AtomicUsize>,
}

impl<E> CQueue<E> {
    /// Returns a String describing the datatype and its parameters.
    pub fn descriptor(&self) -> String {
        format!("CTimeVDeque({}, {:?})", self.n, self.t)
    }

    /// Returns the number of elements in the queue.
    pub fn len(&self) -> usize {
        self.len.load(SeqCst)
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

    /// Returns the timestamp of the last emitted event.
    /// This acts as a lower bound to the insertion of new events.
    pub fn time(&self) -> Duration {
        self.t_current
    }

    /// Creates a new parameteriszed CQueue.
    pub fn new(n: usize, t: Duration) -> Self {
        // essentialy t*n
        let t_all = t.as_nanos() * n as u128;

        let len = Arc::new(AtomicUsize::new(0));

        Self {
            n,
            t_nanos: t.as_nanos(),
            t,

            zero_event_bucket: DLL::new(len.clone()),
            buckets: std::iter::repeat_with(|| DLL::new(len.clone()))
                .take(n)
                .collect(),
            head: 0,

            t_current: Duration::ZERO,

            t0: Duration::ZERO,
            t1: t,

            t_all,

            empty_nodes: std::iter::repeat_with(|| EventNode::empty(Duration::ZERO))
                .take(256)
                .collect(),
            len,
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
        assert!(time >= self.t_current);

        if time == self.t_current {
            let (node, handle) = if let Some(mut node) = self.empty_nodes.pop() {
                node.time = time;
                node.value = Some(event);
                let handle = EventHandle::new(&mut node, self.len.clone());

                (node, handle)
            } else {
                EventNode::new(event, time, self.len.clone())
            };
            self.zero_event_bucket.add_to_tail(node);
            handle
        } else {
            // delta time ?

            let time_mod = time.as_nanos().rem(self.t_all);

            let index = time_mod / self.t_nanos;
            let index: usize = index as usize;
            let index = index % self.n;

            // let index_mod = (index + self.head) % self.n;
            // dbg!(index_mod);

            // find insert pos
            let (node, handle) = if let Some(mut node) = self.empty_nodes.pop() {
                node.time = time;
                node.value = Some(event);
                let handle = EventHandle::new(&mut node, self.len.clone());

                (node, handle)
            } else {
                EventNode::new(event, time, self.len.clone())
            };
            self.buckets[index].add(node);
            handle
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
        assert!(self.len() != 0, "Cannot fetch from empty queue");

        if let Some((ret, node)) = self.zero_event_bucket.pop_min() {
            self.empty_nodes.push(node);
            return ret;
        }

        loop {
            // Move until full bucket is found.
            while self.buckets[self.head].is_empty() {
                self.head = (self.head + 1) % self.n;
                self.t0 += self.t;
                self.t1 += self.t;
            }

            // Bucket with > 0 elements found

            let (_, min) = self.buckets[self.head].front().unwrap();
            if min > self.t1 {
                self.head = (self.head + 1) % self.n;
                self.t0 += self.t;
                self.t1 += self.t;
                continue;
            }

            self.t_current = min;

            let (ret, node) = self.buckets[self.head].pop_min().unwrap();
            self.empty_nodes.push(node);
            return ret;
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
        println!("Num nodes: {} / {}", self.empty_nodes.len(), self.len())
    }
}

#[cfg(test)]
mod tests;
