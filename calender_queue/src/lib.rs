use num_traits::Zero;
use std::cmp::{Ord, PartialOrd};
use std::collections::{BinaryHeap, VecDeque};

#[derive(Debug, Clone)]
pub struct Node<T, A>
where
    T: Zero + Ord,
{
    pub time: T,
    pub value: A,
    pub value_cookie: u64,
}

impl<T, A> PartialEq for Node<T, A>
where
    T: Zero + Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.value_cookie == other.value_cookie
    }
}

impl<T, A> Eq for Node<T, A> where T: Zero + Ord {}

impl<T, A> PartialOrd for Node<T, A>
where
    T: Zero + Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, A> Ord for Node<T, A>
where
    T: Zero + Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.time.cmp(&self.time)
    }
}

pub struct CalenderQueueOptions<T>
where
    T: Zero + Copy + Ord,
{
    pub num_buckets: usize,
    pub bucket_timespan: T,
    pub min_time: T,
}

pub struct CalenderQueue<T, A>
where
    T: Zero + Ord,
{
    n: usize,
    t: T,

    upper_bounds: Vec<T>,

    zero_bucket: VecDeque<Node<T, A>>,
    buckets: Vec<VecDeque<Node<T, A>>>,
    overflow_bucket: BinaryHeap<Node<T, A>>,

    cookie: u64,
    len: usize,
    time: T,
}

impl<T, A> CalenderQueue<T, A>
where
    T: Zero + Copy + Ord,
{
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

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

    pub fn time(&self) -> T {
        self.time
    }

    pub fn len_buckets_filled(&self) -> usize {
        self.buckets
            .iter()
            .enumerate()
            .find(|(_, b)| b.len() == 0)
            .map(|(idx, _)| idx)
            .unwrap_or(self.n)
    }

    pub fn new_with(options: CalenderQueueOptions<T>) -> Self {
        let n = options.num_buckets;
        let t = options.bucket_timespan;

        let mut upper_bounds = Vec::with_capacity(n);
        let mut time = T::zero();
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

    pub fn fetch_next(&mut self) -> Node<T, A> {
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
    pub fn add(&mut self, time: T, value: A) {
        let node = Node {
            time,
            value,
            value_cookie: self.cookie,
        };
        self.cookie += 1;
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

                match self.buckets[i].binary_search_by(|node| node.time.cmp(&time)) {
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
