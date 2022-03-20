#[cfg(not(feature = "cqueue"))]
mod default {
    use crate::{core::event::EventNode, Application, SimTime};
    use std::collections::{BinaryHeap, VecDeque};

    pub(crate) struct FutureEventSet<A>
    where
        A: Application,
    {
        heap: BinaryHeap<EventNode<A>>,
        zero_queue: VecDeque<EventNode<A>>,

        last_event_simtime: SimTime,
    }

    impl<A> FutureEventSet<A>
    where
        A: Application,
    {
        pub fn len(&self) -> usize {
            self.len_zero() + self.len_nonzero()
        }

        pub fn is_empty(&self) -> bool {
            self.heap.is_empty() && self.zero_queue.is_empty()
        }

        pub fn len_zero(&self) -> usize {
            self.zero_queue.len()
        }

        pub fn len_nonzero(&self) -> usize {
            self.heap.len()
        }

        pub fn new() -> Self {
            Self {
                heap: BinaryHeap::with_capacity(64),
                zero_queue: VecDeque::with_capacity(32),

                last_event_simtime: SimTime::ZERO,
            }
        }

        pub fn fetch_next(
            &mut self,
            #[cfg(feature = "internal-metrics")] metrics: &mut Metrics,
        ) -> EventNode<A> {
            // Internal runtime metrics
            #[cfg(feature = "internal-metrics")]
            {
                metrics.record_handled(self);
            }

            if let Some(event) = self.zero_queue.pop_front() {
                #[cfg(feature = "internal-metrics")]
                {
                    metrics.zero_event_count += 1;
                }

                self.last_event_simtime = event.time;
                event
            } else {
                #[cfg(feature = "internal-metrics")]
                {
                    metrics.non_zero_event_count += 1;
                }

                let event = self.heap.pop().unwrap();

                self.last_event_simtime = event.time;
                event
            }
        }

        pub fn add(&mut self, time: SimTime, event: impl Into<A::EventSet>) {
            assert!(
                time >= self.last_event_simtime,
                "Sorry we cannot timetravel yet"
            );

            let node = EventNode::create_no_id(event.into(), time);

            if self.last_event_simtime == time {
                self.zero_queue.push_back(node);
            } else {
                self.heap.push(node);
            }
        }
    }
}

#[cfg(not(feature = "cqueue"))]
pub(crate) use default::*;

#[cfg(feature = "cqueue")]
mod cqueue {
    use crate::{core::event::EventNode, Application, SimTime};
    use std::collections::{BinaryHeap, VecDeque};

    const N: usize = 10;
    const T: SimTime = SimTime::new(0.2);

    pub(crate) struct FutureEventSet<A>
    where
        A: Application,
    {
        upper_bounds: [SimTime; N],

        zero_bucket: VecDeque<EventNode<A>>,
        buckets: Vec<VecDeque<EventNode<A>>>,
        overflow_bucket: BinaryHeap<EventNode<A>>,

        len: usize,
        time: SimTime,
    }

    impl<A> FutureEventSet<A>
    where
        A: Application,
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

        pub fn new() -> Self {
            let mut upper_bounds = [SimTime::ZERO; N];
            let mut time = SimTime::ZERO;
            for i in 0..N {
                upper_bounds[i] = time;
                time += T;
            }

            Self {
                upper_bounds,

                zero_bucket: VecDeque::with_capacity(16),
                buckets: std::iter::repeat_with(|| VecDeque::with_capacity(16))
                    .take(N)
                    .collect(),
                overflow_bucket: BinaryHeap::with_capacity(16),

                len: 0,
                time: SimTime::ZERO,
            }
        }

        pub fn fetch_next(
            &mut self,
            #[cfg(feature = "internal-metrics")] metrics: &mut Metrics,
        ) -> EventNode<A> {
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
                for i in 0..(N - 1) {
                    self.buckets.swap(i, i + 1);
                    self.upper_bounds.swap(i, i + 1);
                }

                // Now at N-1 there is an empty bucket
                // at N there is a inifinte bucket
                assert!(self.buckets[N - 1].is_empty());

                // Set new bound
                let bound = self.upper_bounds[N - 2] + T;
                self.upper_bounds[N - 1] = bound;

                // Filter elements
                while let Some(element) = self.overflow_bucket.peek() {
                    if element.time <= bound {
                        let element = self.overflow_bucket.pop().unwrap();

                        // This is super inefficient
                        self.buckets[N - 1].push_back(element);
                    } else {
                        break;
                    }
                }
            }
        }
        pub fn add(&mut self, time: SimTime, event: impl Into<A::EventSet>) {
            let node = EventNode::create_no_id(event.into(), time);
            self.len += 1;

            // Zero event optimization
            if time == self.time {
                self.zero_bucket.push_back(node);
                return;
            }

            for i in 0..N {
                if time > self.upper_bounds[i] {
                    continue;
                } else {
                    // Insert into finite bucket

                    match self.buckets[i].binary_search_by(|node| time.cmp(&node.time)) {
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
}

#[cfg(feature = "cqueue")]
pub(crate) use cqueue::*;
