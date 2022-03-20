#[cfg(not(feature = "cqueue"))]
mod default {
    use crate::{core::event::EventNode, Application, RuntimeOptions, SimTime};
    use std::collections::{BinaryHeap, VecDeque};

    #[cfg(feature = "internal-metrics")]
    use crate::Statistic;

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

        pub fn new_with(_options: &RuntimeOptions) -> Self {
            Self {
                heap: BinaryHeap::with_capacity(64),
                zero_queue: VecDeque::with_capacity(32),

                last_event_simtime: SimTime::ZERO,
            }
        }

        pub fn fetch_next(
            &mut self,
            #[cfg(feature = "internal-metrics")] mut metrics: crate::Mrc<
                crate::metrics::RuntimeMetrics,
            >,
        ) -> EventNode<A> {
            // Internal runtime metrics

            let event = if let Some(event) = self.zero_queue.pop_front() {
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
            };

            #[cfg(feature = "internal-metrics")]
            {
                metrics
                    .heap_size
                    .collect_at(self.len_nonzero() as f64, event.time);

                let total = self.len() + 1;
                let perc = self.len_zero() as f64 / total as f64;
                metrics.zero_event_prec.collect_at(perc, event.time);
            }

            event
        }

        pub fn add(
            &mut self,
            time: SimTime,
            event: impl Into<A::EventSet>,
            #[cfg(feature = "internal-metrics")] mut metrics: crate::Mrc<
                crate::metrics::RuntimeMetrics,
            >,
        ) {
            assert!(
                time >= self.last_event_simtime,
                "Sorry we cannot timetravel yet"
            );

            let node = EventNode::create_no_id(event.into(), time);

            if self.last_event_simtime == time {
                self.zero_queue.push_back(node);
            } else {
                #[cfg(feature = "internal-metrics")]
                metrics
                    .non_zero_event_wait_time
                    .collect_at((time - SimTime::now()).into(), SimTime::now());

                self.heap.push(node);
            }
        }
    }
}

#[cfg(not(feature = "cqueue"))]
pub(crate) use default::*;

#[cfg(feature = "cqueue")]
mod cqueue {
    use crate::{core::event::EventNode, Application, RuntimeOptions, SimTime};
    use std::collections::{BinaryHeap, VecDeque};

    #[cfg(feature = "internal-metrics")]
    use std::ops::AddAssign;

    #[cfg(feature = "internal-metrics")]
    use crate::Statistic;

    pub(crate) struct FutureEventSet<A>
    where
        A: Application,
    {
        n: usize,
        t: SimTime,

        upper_bounds: Vec<SimTime>,

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

        pub fn new_with(options: &RuntimeOptions) -> Self {
            let n = options.cqueue_num_buckets;
            let t = options.cqueue_bucket_timespan;

            let mut upper_bounds = Vec::with_capacity(n);
            let mut time = SimTime::ZERO;
            for _ in 0..n {
                upper_bounds.push(time);
                time += t;
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

                len: 0,
                time: SimTime::ZERO,
            }
        }

        pub fn fetch_next(
            &mut self,
            #[cfg(feature = "internal-metrics")] mut metrics: crate::Mrc<
                crate::metrics::RuntimeMetrics,
            >,
        ) -> EventNode<A> {
            assert!(!self.is_empty());

            // Zero event optimization
            if let Some(event) = self.zero_bucket.pop_front() {
                #[cfg(feature = "internal-metrics")]
                metrics.zero_event_count.add_assign(1);

                self.len -= 1;

                event
            } else {
                // Assure that the eralies bucket contains elements
                self.cleanup_empty_buckets();

                #[cfg(feature = "internal-metrics")]
                metrics.nonzero_event_count.add_assign(1);

                let event = self.buckets[0].pop_front().unwrap();
                self.len -= 1;
                self.time = event.time;

                #[cfg(feature = "internal-metrics")]
                {
                    metrics
                        .overflow_heap_size
                        .collect_at(self.overflow_bucket.len() as f64, event.time);
                    metrics.queue_bucket_size.collect_at(
                        (self.len_nonzero() - self.overflow_bucket.len()) as f64,
                        event.time,
                    );

                    metrics
                        .avg_first_bucket_fill
                        .collect_at((self.buckets[0].len() + 1usize) as f64, event.time);
                    metrics.avg_filled_buckets.collect_at(
                        self.buckets
                            .iter()
                            .enumerate()
                            .find(|(_, b)| b.len() == 0)
                            .map(|(idx, _)| idx)
                            .unwrap_or(N) as f64,
                        event.time,
                    );
                }

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
        pub fn add(
            &mut self,
            time: SimTime,
            event: impl Into<A::EventSet>,
            #[cfg(feature = "internal-metrics")] mut metrics: crate::Mrc<
                crate::metrics::RuntimeMetrics,
            >,
        ) {
            let node = EventNode::create_no_id(event.into(), time);
            self.len += 1;

            // Zero event optimization
            if time == self.time {
                self.zero_bucket.push_back(node);
                return;
            }

            // Messure the avg age of events.
            #[cfg(feature = "internal-metrics")]
            metrics
                .non_zero_event_wait_time
                .collect_at((time - SimTime::now()).into(), SimTime::now());

            for i in 0..self.n {
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
