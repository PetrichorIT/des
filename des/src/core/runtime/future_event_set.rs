#[cfg(not(feature = "cqueue"))]
mod default {
    #[allow(unused)]
    use crate::{
        core::{
            event::{Application, EventNode},
            RuntimeOptions, SimTime,
        },
        metrics::*,
        util::*,
    };
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
        pub fn descriptor(&self) -> String {
            format!("FutureEventSet::BinaryHeap()")
        }

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

        pub fn new_with(options: &RuntimeOptions) -> Self {
            Self {
                heap: BinaryHeap::with_capacity(64),
                zero_queue: VecDeque::with_capacity(32),

                last_event_simtime: options.min_sim_time.unwrap_or(SimTime::MIN),
            }
        }

        //
        // clippy::let_and_return occures on not(feature = "internal-metrics")
        // but would produce invalid code with feature "internal-metrics"
        //
        #[allow(clippy::let_and_return)]
        pub fn fetch_next(
            &mut self,
            #[cfg(feature = "internal-metrics")] mut metrics: crate::util::PtrMut<
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
            #[cfg(feature = "internal-metrics")] mut metrics: crate::util::PtrMut<
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
    use std::marker::PhantomData;

    #[allow(unused)]
    use crate::{
        core::{
            event::{Application, EventNode},
            RuntimeOptions, SimTime,
        },
        metrics::*,
        util::*,
    };
    use cqueue::*;

    pub(crate) struct FutureEventSet<A>
    where
        A: Application,
    {
        inner: CQueue<SimTime, A::EventSet>,
    }

    impl<A> FutureEventSet<A>
    where
        A: Application,
    {
        pub fn descriptor(&self) -> String {
            format!("FutureEventSet::CQueue::{}", self.inner.descriptor())
        }

        pub fn len(&self) -> usize {
            self.inner.len()
        }

        pub fn len_nonzero(&self) -> usize {
            // self.inner.len_nonzero()
            todo!();
        }

        pub fn len_zero(&self) -> usize {
            // self.inner.len_zero()
            todo!();
        }

        pub fn is_empty(&self) -> bool {
            self.inner.is_empty()
        }

        pub fn new_with(options: &RuntimeOptions) -> Self {
            let cqueue_options = CQueueOptions {
                num_buckets: options.cqueue_num_buckets,
                bucket_timespan: options.cqueue_bucket_timespan,

                ..Default::default()
            };

            Self {
                inner: CQueue::new(cqueue_options),
            }
        }

        #[inline]
        pub fn fetch_next(
            &mut self,
            #[cfg(feature = "internal-metrics")] mut metrics: PtrMut<RuntimeMetrics>,
        ) -> EventNode<A> {
            #[cfg(feature = "internal-metrics")]
            {
                use std::ops::AddAssign;
                let is_zero_time = self.inner.len_zero() > 0;
                if is_zero_time {
                    metrics.zero_event_count.add_assign(1);
                } else {
                    metrics.nonzero_event_count.add_assign(1);
                }
            }

            let Node {
                time,
                event,
                cookie,
                ..
            } = self.inner.fetch_next();

            #[cfg(feature = "internal-metrics")]
            {
                // metrics
                //    .overflow_heap_size
                //    .collect_at(self.inner.len_overflow() as f64, time);
                // metrics.queue_bucket_size.collect_at(
                //     (self.inner.len_nonzero() - self.inner.len_overflow()) as f64,
                //     time,
                // );
                // metrics
                //    .avg_first_bucket_fill
                //    .collect_at((self.inner.len_first_bucket() + 1usize) as f64, time);
                //
                // metrics
                //     .avg_filled_buckets
                //    .collect_at(self.inner.len_buckets_filled() as f64, time);
            }

            EventNode {
                time,
                id: cookie,
                event: event,

                _phantom: PhantomData,
            }
        }

        pub fn add(
            &mut self,
            time: SimTime,
            event: impl Into<A::EventSet>,
            #[cfg(feature = "internal-metrics")] mut metrics: PtrMut<RuntimeMetrics>,
        ) {
            #[cfg(feature = "internal-metrics")]
            {
                if time > self.inner.time() {
                    metrics
                        .non_zero_event_wait_time
                        .collect_at((time - SimTime::now()).into(), SimTime::now());
                }
            }

            self.inner.add(time, event.into())
        }
    }
}

#[cfg(feature = "cqueue")]
pub(crate) use self::cqueue::*;
