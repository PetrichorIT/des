cfg_not_cqueue! {
    mod default_impl {
        #[allow(unused)]
        use crate::{stats::*, runtime::*, time::SimTime, util::*};
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
            pub(crate) fn descriptor(&self) -> String {
                "FutureEventSet::BinaryHeap()".to_string()
            }

            pub(crate) fn len(&self) -> usize {
                self.len_zero() + self.len_nonzero()
            }

            pub(crate) fn is_empty(&self) -> bool {
                self.heap.is_empty() && self.zero_queue.is_empty()
            }

            pub(crate) fn len_zero(&self) -> usize {
                self.zero_queue.len()
            }

            pub(crate) fn len_nonzero(&self) -> usize {
                self.heap.len()
            }

            pub(crate) fn new_with(options: &RuntimeOptions) -> Self {
                Self {
                    heap: BinaryHeap::with_capacity(64),
                    zero_queue: VecDeque::with_capacity(32),

                    last_event_simtime: options.min_sim_time.unwrap_or(SimTime::MIN),
                }
            }

            //
            // clippy::let_and_return occures on not(feature = "metrics")
            // but would produce invalid code with feature "metrics"
            //
            #[allow(clippy::let_and_return)]
            pub(crate) fn fetch_next(
                &mut self,
                #[cfg(feature = "metrics")] mut metrics: crate::util::PtrMut<
                    crate::stats::RuntimeMetrics,
                >,
            ) -> EventNode<A> {
                // Internal runtime metrics

                let event = if let Some(event) = self.zero_queue.pop_front() {
                    #[cfg(feature = "metrics")]
                    {
                        metrics.zero_event_count += 1;
                    }

                    self.last_event_simtime = event.time;
                    event
                } else {
                    #[cfg(feature = "metrics")]
                    {
                        metrics.non_zero_event_count += 1;
                    }

                    let event = self.heap.pop().unwrap();

                    self.last_event_simtime = event.time;
                    event
                };

                #[cfg(feature = "metrics")]
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

            pub(crate) fn add(
                &mut self,
                time: SimTime,
                event: impl Into<A::EventSet>,
                #[cfg(feature = "metrics")] mut metrics: crate::util::PtrMut<
                    crate::stats::RuntimeMetrics,
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
                    #[cfg(feature = "metrics")]
                    metrics
                        .non_zero_event_wait_time
                        .collect_at((time - SimTime::now()).as_secs_f64(), SimTime::now());

                    self.heap.push(node);
                }
            }
        }
    }

    pub(crate) use default_impl::*;
}

cfg_cqueue! {
    mod cqueue_impl {
        #[allow(unused)]
        use super::*;
        use std::marker::PhantomData;

        #[allow(unused)]
        use crate::{stats::*, runtime::*, time::*, util::*};
        use crate::cqueue::*;

        pub(crate) struct FutureEventSet<A>
        where
            A: Application,
        {
            inner: CQueue<A::EventSet>,
        }

        impl<A> FutureEventSet<A>
        where
            A: Application,
        {
            pub(crate) fn descriptor(&self) -> String {
                format!("FutureEventSet::CQueue::{}", self.inner.descriptor())
            }

            pub(crate) fn len(&self) -> usize {
                self.inner.len()
            }

            pub(crate) fn len_nonzero(&self) -> usize {
                // self.inner.len_nonzero()
                todo!();
            }

            pub(crate) fn len_zero(&self) -> usize {
                // self.inner.len_zero()
                todo!();
            }

            pub(crate) fn is_empty(&self) -> bool {
                self.inner.is_empty()
            }

            pub(crate) fn new_with(options: &RuntimeOptions) -> Self {
                let cqueue_options = CQueueOptions {
                    num_buckets: options.cqueue_num_buckets,
                    bucket_timespan: options.cqueue_bucket_timespan,
                };

                Self {
                    inner: CQueue::new(cqueue_options),
                }
            }

            #[inline]
            pub(crate) fn fetch_next(
                &mut self,
                #[cfg(feature = "metrics")] mut metrics: PtrMut<RuntimeMetrics>,
            ) -> EventNode<A> {
                #[cfg(feature = "metrics")]
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

                #[cfg(feature = "metrics")]
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
                    event,

                    _phantom: PhantomData,
                }
            }

            pub(crate) fn add(
                &mut self,
                time: SimTime,
                event: impl Into<A::EventSet>,
                #[cfg(feature = "metrics")] mut metrics: PtrMut<RuntimeMetrics>,
            ) {
                #[cfg(feature = "metrics")]
                {
                    if time > self.inner.time() {
                        metrics
                            .non_zero_event_wait_time
                            .collect_at((time - SimTime::now()).as_secs_f64(), SimTime::now());
                    }
                }

                self.inner.add(time, event.into())
            }
        }
    }

    pub(crate) use self::cqueue_impl::*;

}
