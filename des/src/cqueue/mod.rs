#![allow(unused)]

use crate::time::*;
use std::collections::VecDeque;
use std::ops::Rem;

#[derive(Debug, Clone)]
pub struct Node<E> {
    pub time: SimTime,
    pub event: E,

    pub cookie: usize,
}

impl<E> PartialEq for Node<E> {
    fn eq(&self, other: &Self) -> bool {
        self.cookie == other.cookie
    }
}

impl<E> Eq for Node<E> {}

impl<E> PartialOrd for Node<E>
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<E> Ord for Node<E>
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse for min
        other
            .time
            .partial_cmp(&self.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

pub struct CQueueOptions {
    pub num_buckets: usize,
    pub bucket_timespan: Duration,
}

impl Default for CQueueOptions

{
    fn default() -> Self {
        Self {
            num_buckets: 30,
            bucket_timespan: Duration::new(1, 0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CQueue<E> {
    // Parameters
    pub(crate) n: usize,
    pub(crate) t: Duration,

    pub(crate) t_nanos: u128,

    // Buckets
    pub(crate) zero_event_bucket: VecDeque<Node<E>>,

    pub(crate) buckets: Vec<VecDeque<Node<E>>>,
    pub(crate) head: usize,

    pub(crate) t_current: SimTime,

    pub(crate) t0: SimTime,
    pub(crate) t1: SimTime,
    pub(crate) t_all: u128,

    // Misc
    pub(crate) len: usize,
    pub(crate) running_cookie: usize,
}

impl<E> CQueue<E>
{
    pub fn descriptor(&self) -> String
    {
        format!("CTimeVDeque({}, {:?})", self.n, self.t)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn len_zero(&self) -> usize {
        self.zero_event_bucket.len()
    }

    pub fn len_nonzero(&self) -> usize {
        self.len() - self.len_zero()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn time(&self) -> SimTime {
        self.t_current
    }

    pub fn new(options: CQueueOptions) -> Self {
        let CQueueOptions {
            num_buckets: n,
            bucket_timespan: t,
        } = options;

        // essentialy t*n
        let t_all = t.as_nanos() * n as u128;

        Self {
            n,
            t_nanos: t.as_nanos(),
            t,

            zero_event_bucket: VecDeque::with_capacity(16),
            buckets: std::iter::repeat_with(|| VecDeque::with_capacity(16))
                .take(n)
                .collect(),
            head: 0,

            t_current: SimTime::ZERO,

            t0: SimTime::ZERO,
            t1: SimTime::from_duration(t),

            t_all,

            len: 0,
            running_cookie: 0,
        }
    }

    #[inline]
    pub fn add(&mut self, time: SimTime, event: E) {
        self.enqueue(time, event)
    }

    pub fn enqueue(&mut self, time: SimTime, event: E) {
        assert!(time >= self.t_current);

        let node = Node {
            time,
            event,
            cookie: self.running_cookie,
        };
        self.running_cookie = self.running_cookie.wrapping_add(1);

        if time == self.t_current {
            self.zero_event_bucket.push_back(node);
            self.len += 1;
            return;
        }

        // delta time ?

        let time_mod = time.as_nanos().rem(self.t_all);

        let index = time_mod / self.t_nanos;
        let index: usize = index as usize;
        let index = index % self.n;

        // let index_mod = (index + self.head) % self.n;
        // dbg!(index_mod);

        // find insert pos
        match self.buckets[index].binary_search_by(|node| node.time.partial_cmp(&time).unwrap()) {
            Ok(mut idx) => {
                // A event at the same time allready exits
                // thus make sure the ord is right;

                // Order is important to shortciruit
                while idx < self.buckets[index].len() && self.buckets[index][idx].time == time {
                    idx += 1;
                }

                idx -= 1;

                self.buckets[index].insert(idx, node);
            }
            Err(idx) => {
                // New timestamp
                self.buckets[index].insert(idx, node);
            }
        }
        self.len += 1;
    }

    #[inline]
    pub fn fetch_next(&mut self) -> Node<E> {
        if self.len == 0 {
            panic!("Cannot fetch from empty queue");
        }

        if let Some(node) = self.zero_event_bucket.pop_front() {
            self.len -= 1;
            return node;
        }

        loop {
            // Move until full bucket is found.
            while self.buckets[self.head].is_empty() {
                self.head = (self.head + 1) % self.n;
                self.t0 = self.t0 + self.t;
                self.t1 = self.t1 + self.t;
            }

            // Bucket with > 0 elements found

            let min = self.buckets[self.head].front().unwrap();
            if min.time > self.t1 {
                self.head = (self.head + 1) % self.n;
                self.t0 = self.t0 + self.t;
                self.t1 = self.t1 + self.t;
                continue;
            }

            self.t_current = min.time;

            self.len -= 1;
            return self.buckets[self.head].pop_front().unwrap();
        }
    }

    pub fn dequeue(&mut self) -> Option<Node<E>> {
        if self.len == 0 {
            return None;
        }

        if let Some(node) = self.zero_event_bucket.pop_front() {
            self.len -= 1;
            return Some(node);
        }

        loop {
            // Move until full bucket is found.
            while self.buckets[self.head].is_empty() {
                self.head = (self.head + 1) % self.n;
                self.t0 = self.t0 + self.t;
                self.t1 = self.t1 + self.t;
            }

            // Bucket with > 0 elements found

            let min = self.buckets[self.head].front().unwrap();
            if min.time > self.t1 {
                self.head = (self.head + 1) % self.n;
                self.t0 = self.t0 + self.t;
                self.t1 = self.t1 + self.t;
                continue;
            }

            self.t_current = min.time;

            self.len -= 1;
            return self.buckets[self.head].pop_front();
        }
    }

    // pub fn clear(&mut self) {
    //     self.zero_event_bucket.clear();
    //     self.buckets.iter_mut().for_each(VecDeque::clear);
    //     self.len = 0;
    //     self.head = 0;
    // }

    // pub fn reset(&mut self, time: SimTime) {
    //     self.clear();

    //     self.t_current = time;
    //     self.t0 = time;

    //     self.t1 = time + self.t;
    //     // t_all remains the same
    //     self.running_cookie = 0;
    // }
}

