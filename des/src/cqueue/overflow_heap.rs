use std::cmp::{Ord, PartialOrd};
use std::collections::{BinaryHeap, VecDeque};
use std::fmt::Debug;

use crate::prelude::Duration;
use crate::time::SimTime;

#[derive(Debug, Clone)]
pub struct Node<A> {
    pub time: SimTime,
    pub event: A,
    pub cookie: usize,
}

impl<A> PartialEq for Node<A> {
    fn eq(&self, other: &Self) -> bool {
        self.cookie == other.cookie
    }
}

impl<A> Eq for Node<A> {}

impl<A> PartialOrd for Node<A> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<A> Ord for Node<A> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .time
            .partial_cmp(&self.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

pub struct CQueueOptions {
    pub num_buckets: usize,
    pub bucket_timespan: Duration,
    pub bucket_capacity: usize,

    pub overflow_capacity: usize,

    pub min_time: SimTime,
}

impl Default for CQueueOptions {
    fn default() -> Self {
        Self {
            num_buckets: 10,
            bucket_timespan: Duration::new(1, 0),
            bucket_capacity: 16,

            overflow_capacity: 16,

            min_time: SimTime::MIN,
        }
    }
}

pub struct CQueue<A> {
    n: usize,
    t: Duration,

    upper_bounds: Vec<SimTime>,

    zero_bucket: VecDeque<Node<A>>,
    buckets: Vec<VecDeque<Node<A>>>,
    overflow_bucket: BinaryHeap<Node<A>>,

    cookie: usize,
    len: usize,
    time: SimTime,
}

impl<A> CQueue<A> {
    pub fn descriptor(&self) -> String {
        format!("OverflowHeapNBucket({}, {:?})", self.n, self.t)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn new(options: CQueueOptions) -> Self {
        let n = options.num_buckets;
        let t = options.bucket_timespan;

        let mut upper_bounds = Vec::with_capacity(n);
        let mut time = SimTime::ZERO;
        for _ in 0..n {
            upper_bounds.push(time);
            time = time + t;
        }

        Self {
            n,
            t,

            upper_bounds,

            zero_bucket: VecDeque::with_capacity(16),
            buckets: std::iter::repeat_with(|| VecDeque::with_capacity(16))
                .take(n)
                .collect(),
            overflow_bucket: BinaryHeap::with_capacity(16),

            cookie: 0,
            len: 0,
            time: options.min_time,
        }
    }

    // pub(crate) fn clear(&mut self) {
    //     self.buckets.iter_mut().for_each(|b| b.clear());
    //     self.zero_bucket.clear();
    //     self.overflow_bucket.clear();
    //     self.len = 0;
    // }

    // pub(crate) fn reset(&mut self, time: SimTime) {
    //     self.clear();
    //     self.time = time;
    //     // no cookie reset

    //     let mut time = self.time;
    //     for i in 0..self.n {
    //         self.upper_bounds[i] = time;
    //         time = time + self.t;
    //     }
    // }

    pub fn fetch_next(&mut self) -> Node<A> {
        assert!(!self.is_empty());

        // Zero event optimization
        if let Some(event) = self.zero_bucket.pop_front() {
            self.len -= 1;

            event
        } else {
            // Assure that the eralies bucket contains elements
            self.cleanup_empty_buckets();

            let event = self.buckets[0].pop_front().unwrap();
            self.len -= 1;
            self.time = event.time;

            event
        }
    }

    #[inline(always)]
    fn cleanup_empty_buckets(&mut self) {
        assert!(!self.is_empty());

        // Check for empty buckets
        while self.buckets[0].is_empty() {
            // Shift up all finite buckets
            for i in 0..(self.n - 1) {
                self.buckets.swap(i, i + 1);
                self.upper_bounds.swap(i, i + 1);
            }

            // Now at N-1 there is an empty bucket
            // at N there is a inifinte bucket
            assert!(self.buckets[self.n - 1].is_empty());

            // Set new bound
            let bound = self.upper_bounds[self.n - 2] + self.t;
            self.upper_bounds[self.n - 1] = bound;

            // Filter elements
            while let Some(element) = self.overflow_bucket.peek() {
                if element.time <= bound {
                    let element = self.overflow_bucket.pop().unwrap();

                    // This is super inefficient
                    self.buckets[self.n - 1].push_back(element);
                } else {
                    break;
                }
            }
        }
    }
    pub fn add(&mut self, time: SimTime, value: A) {
        let node = Node {
            time,
            event: value,
            cookie: self.cookie,
        };
        self.cookie = self.cookie.wrapping_add(1);
        self.len += 1;

        // Zero event optimization
        if time == self.time {
            self.zero_bucket.push_back(node);
            return;
        }

        for i in 0..self.n {
            if time > self.upper_bounds[i] {
                continue;
            } else {
                // Insert into finite bucket

                match self.buckets[i].binary_search_by(|node| node.time.partial_cmp(&time).unwrap())
                {
                    Ok(mut idx) => {
                        // A event at the same time allready exits
                        // thus make sure the ord is right;

                        // Order is important to shortciruit
                        while idx < self.buckets[i].len() && self.buckets[i][idx].time == time {
                            idx += 1;
                        }

                        idx -= 1;

                        self.buckets[i].insert(idx, node);
                    }
                    Err(idx) => {
                        // New timestamp
                        self.buckets[i].insert(idx, node);
                    }
                }

                return;
            }
        }

        // insert into infinite bucket
        self.overflow_bucket.push(node)
    }
}

cfg_metrics! {
    #[allow(unused)]
    impl<A> CQueue<A> {
        pub fn len_zero(&self) -> usize {
            self.zero_bucket.len()
        }

        pub fn len_nonzero(&self) -> usize {
            self.len - self.zero_bucket.len()
        }

        pub fn len_overflow(&self) -> usize {
            self.overflow_bucket.len()
        }

        pub fn len_first_bucket(&self) -> usize {
            self.buckets[0].len()
        }

        pub fn time(&self) -> SimTime {
            self.time
        }

        pub fn len_buckets_filled(&self) -> usize {
            self.buckets
                .iter()
                .enumerate()
                .find(|(_, b)| b.is_empty())
                .map(|(idx, _)| idx)
                .unwrap_or(self.n)
        }
    }
}

impl<A> Debug for CQueue<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalenderQueue")
            .field("n", &self.n)
            .field("t", &self.t)
            .field("bounds", &self.upper_bounds)
            .field("time", &self.time)
            .finish()
    }
}
