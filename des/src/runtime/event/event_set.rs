cfg_not_cqueue! {
    mod default_impl {
        use crate::{runtime::{Application, EventNode}, time::SimTime};
        use std::collections::{BinaryHeap, VecDeque};
        use crate::runtime::Builder;


        #[cfg(feature = "metrics")]
        use crate::stats::{Statistic, RuntimeMetrics};
        #[cfg(feature = "metrics")]
        use std::{cell::RefCell, sync::Arc};

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
            #[allow(clippy::unused_self)]
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

            pub(crate) fn new_with(options: &Builder) -> Self {
                Self {
                    heap: BinaryHeap::with_capacity(64),
                    zero_queue: VecDeque::with_capacity(32),

                    last_event_simtime: options.start_time,
                }
            }

            //
            // clippy::let_and_return occures on not(feature = "metrics")
            // but would produce invalid code with feature "metrics"
            //
            #[allow(clippy::let_and_return)]
            #[allow(clippy::needless_pass_by_value)]
            #[allow(clippy::cast_precision_loss)]
            pub(crate) fn fetch_next(
                &mut self,
                #[cfg(feature = "metrics")]  metrics: Arc<RefCell<RuntimeMetrics>>,
            ) -> (A::EventSet, SimTime) {
                // Internal runtime metrics
                #[cfg(feature = "metrics")]
                let mut metrics = metrics.borrow_mut();

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

                (event.event, event.time)
            }

            #[allow(clippy::needless_pass_by_value)]
            pub(crate) fn add(
                &mut self,
                time: SimTime,
                event: impl Into<A::EventSet>,
                #[cfg(feature = "metrics")]  metrics: Arc<RefCell<RuntimeMetrics>>,
            ) {
                assert!(
                    time >= self.last_event_simtime,
                    "Sorry we cannot timetravel yet"
                );

                let node = EventNode {
                    id: 0,
                    event: event.into(),
                    time,

                    _phantom: std::marker::PhantomData,
                };

                if self.last_event_simtime == time {
                    self.zero_queue.push_back(node);
                } else {
                    #[cfg(feature = "metrics")]
                    metrics.borrow_mut()
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
        #[cfg(feature = "metrics")]
        use std::{sync::Arc, cell::RefCell};
        #[cfg(feature = "metrics")]
        use crate::stats::{Statistic, RuntimeMetrics};


        use crate::{runtime::{Application, Builder}, time::SimTime};
        use des_cqueue::CQueue;



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
            #[allow(clippy::unused_self)]
            pub(crate) fn descriptor(&self) -> String {
                format!("FutureEventSet::CQueue::{}", self.inner.descriptor())
            }

            pub(crate) fn len(&self) -> usize {
                self.inner.len()
            }

            // pub(crate) fn len_nonzero(&self) -> usize {
            //     // self.inner.len_nonzero()
            //     self.inner.len_nonzero()
            // }

            // pub(crate) fn len_zero(&self) -> usize {
            //     // self.inner.len_zero()
            //     self.inner.len_zero()
            // }

            pub(crate) fn is_empty(&self) -> bool {
                self.inner.is_empty()
            }

            pub(crate) fn new_with(options: &Builder) -> Self {
                Self {
                    inner: CQueue::new(options.cqueue_num_buckets, options.cqueue_bucket_timespan),
                }
            }

            #[allow(clippy::needless_pass_by_value)]
            pub(crate) fn fetch_next(
                &mut self,
                #[cfg(feature = "metrics")]  metrics: Arc<RefCell<RuntimeMetrics>>,
            ) -> (A::EventSet, SimTime) {

                #[cfg(feature = "metrics")]
                let mut metrics = metrics.borrow_mut();

                #[cfg(feature = "metrics")]
                #[allow(clippy::cast_precision_loss)]
                {
                    use std::ops::AddAssign;
                    let is_zero_time = self.inner.len_zero() > 0;
                    metrics.zero_queue_size.collect(self.inner.len_zero() as f64);
                    metrics.bucket_queue_size.collect(self.inner.len_nonzero() as f64);

                    if is_zero_time {
                        metrics.zero_event_count.add_assign(1);
                    } else {
                        metrics.nonzero_event_count.add_assign(1);
                    }

                    #[cfg(feature = "metrics-rt-full")]
                    #[allow(clippy::cast_precision_loss)]
                    metrics.event_count.collect(self.len() as f64);

                    #[cfg(feature = "metrics-rt-full")]
                    {
                        let (used, total) = self.inner.metrics();
                        metrics.cqueue_memory_used.collect(used as f64);
                        metrics.cqueue_memory_total.collect(total as f64);
                    }
                }

                let (event, time) = self.inner.fetch_next();

                // #[cfg(feature = "metrics")]
                // {
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
                // }

                (event, SimTime::from_duration(time))
            }

            #[allow(clippy::needless_pass_by_value)]
            pub(crate) fn add(
                &mut self,
                time: SimTime,
                event: impl Into<A::EventSet>,
                #[cfg(feature = "metrics")]  metrics: Arc<RefCell<RuntimeMetrics>>,
            ) {
                #[cfg(feature = "metrics")]
                {
                    if time > SimTime::from_duration(self.inner.time()) {
                        metrics.borrow_mut()
                            .non_zero_event_wait_time
                            .collect_at((time - SimTime::now()).as_secs_f64(), SimTime::now());
                    }
                }

                self.inner.add(*time, event.into());
            }
        }
    }

    pub(crate) use self::cqueue_impl::*;

}
