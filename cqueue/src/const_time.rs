use crate::TimeLike;
use num_traits::One;
use std::collections::VecDeque;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct Node<T, E> {
    pub time: T,
    pub event: E,

    pub cookie: usize,
}

impl<T, E> PartialEq for Node<T, E> {
    fn eq(&self, other: &Self) -> bool {
        self.cookie == other.cookie
    }
}

impl<T, E> Eq for Node<T, E> {}

impl<T, E> PartialOrd for Node<T, E>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, E> Ord for Node<T, E>
where
    T: PartialOrd,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse for min
        other
            .time
            .partial_cmp(&self.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

pub struct CQueueOptions<T> {
    pub num_buckets: usize,
    pub bucket_timespan: T,
}

impl<T> Default for CQueueOptions<T>
where
    T: One,
{
    fn default() -> Self {
        Self {
            num_buckets: 30,
            bucket_timespan: T::one(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CQueue<T, E> {
    // Parameters
    pub(crate) n: usize,
    pub(crate) t: T,

    // Buckets
    pub(crate) zero_event_bucket: VecDeque<Node<T, E>>,

    pub(crate) buckets: Vec<VecDeque<Node<T, E>>>,
    pub(crate) head: usize,

    pub(crate) t_current: T,

    pub(crate) t0: T,
    pub(crate) t1: T,
    pub(crate) t_all: T,

    // Misc
    pub(crate) len: usize,
    pub(crate) running_cookie: usize,
}

impl<T, E> CQueue<T, E>
where
    T: TimeLike,
{
    pub fn descriptor(&self) -> String
    where
        T: Display,
    {
        format!("CTimeVDeque({}, {})", self.n, self.t)
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

    pub fn time(&self) -> T {
        self.t_current
    }

    pub fn new(options: CQueueOptions<T>) -> Self {
        let CQueueOptions {
            num_buckets: n,
            bucket_timespan: t,
        } = options;

        // essentialy t*n
        let mut t_all = t;
        for _ in 1..n {
            t_all = t_all + t;
        }

        Self {
            n,
            t,

            zero_event_bucket: VecDeque::with_capacity(16),
            buckets: std::iter::repeat_with(|| VecDeque::with_capacity(16))
                .take(n)
                .collect(),
            head: 0,

            t_current: T::zero(),

            t0: T::zero(),
            t1: t,

            t_all,

            len: 0,
            running_cookie: 0,
        }
    }

    #[inline]
    pub fn add(&mut self, time: T, event: E) {
        self.enqueue(time, event)
    }

    pub fn enqueue(&mut self, time: T, event: E) {
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

        let time_mod = time.rem(self.t_all);

        let index = time_mod / self.t;
        let index: usize = index.as_usize();
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
    pub fn fetch_next(&mut self) -> Node<T, E> {
        self.dequeue().unwrap()
    }

    pub fn dequeue(&mut self) -> Option<Node<T, E>> {
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

    pub fn clear(&mut self) {
        self.zero_event_bucket.clear();
        self.buckets.iter_mut().for_each(VecDeque::clear);
        self.len = 0;
        self.head = 0;
    }

    pub fn reset(&mut self, time: T) {
        self.clear();

        self.t_current = time;
        self.t0 = time;

        self.t1 = time + self.t;
        // t_all remains the same
        self.running_cookie = 0;
    }
}

impl TimeLike for f32 {
    fn as_usize(self) -> usize {
        self as usize
    }

    fn min(self, other: Self) -> Self {
        f32::min(self, other)
    }
}

impl TimeLike for f64 {
    fn as_usize(self) -> usize {
        self as usize
    }

    fn min(self, other: Self) -> Self {
        f64::min(self, other)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn f64_test() {
        use super::*;

        let mut cqueue = CQueue::new(CQueueOptions {
            num_buckets: 10,
            bucket_timespan: 1.0,
        });
        cqueue.enqueue(12.62, "event");

        cqueue.enqueue(6.62, "event");

        dbg!(cqueue.dequeue());

        cqueue.enqueue(7.62, "event");
        cqueue.enqueue(16.62, "event");

        dbg!(cqueue.dequeue());
        dbg!(cqueue.dequeue());
        dbg!(cqueue.dequeue());
    }
}
