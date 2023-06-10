cfg_not_cqueue! {
    mod default_impl {
        use crate::{runtime::{Application, EventNode}, time::SimTime};
        use std::collections::{BinaryHeap, VecDeque};
        use crate::runtime::Builder;



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
            ) -> (A::EventSet, SimTime) {

                let event = if let Some(event) = self.zero_queue.pop_front() {
                    self.last_event_simtime = event.time;
                    event
                } else {
                    let event = self.heap.pop().unwrap();
                    self.last_event_simtime = event.time;
                    event
                };

                (event.event, event.time)
            }

            #[allow(clippy::needless_pass_by_value)]
            pub(crate) fn add(
                &mut self,
                time: SimTime,
                event: impl Into<A::EventSet>,
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
                    self.heap.push(node);
                }
            }
        }
    }

    pub(crate) use default_impl::*;
}

cfg_cqueue! {
    mod cqueue_impl {
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
            ) -> (A::EventSet, SimTime) {

                let (event, time) = self.inner.fetch_next();
                (event, SimTime::from_duration(time))
            }

            #[allow(clippy::needless_pass_by_value)]
            pub(crate) fn add(
                &mut self,
                time: SimTime,
                event: impl Into<A::EventSet>,
            ) {
                self.inner.add(*time, event.into());
            }
        }
    }

    pub(crate) use self::cqueue_impl::*;

}
