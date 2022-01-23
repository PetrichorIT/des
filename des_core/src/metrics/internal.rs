use std::ops::Sub;

use crate::core::{Application, Runtime, SimTime};
use crate::metrics::StdDev;
use crate::Statistic;

pub struct RuntimeMetrics {
    pub heap_size: StdDev,
    pub non_zero_event_wait_time: StdDev,

    pub zero_event_count: u64,
    pub non_zero_event_count: u64,
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            heap_size: StdDev::new(),
            non_zero_event_wait_time: StdDev::new(),
            zero_event_count: 0,
            non_zero_event_count: 0,
        }
    }

    pub fn record_non_zero_queud<A: Application>(
        &mut self,
        rt: *const Runtime<A>,
        event_time: SimTime,
    ) {
        let rt = unsafe { &*rt };
        self.non_zero_event_wait_time
            .collect_at(event_time.sub(rt.sim_time()).into(), rt.sim_time());
    }

    pub fn record_handled<A: Application>(&mut self, rt: *const Runtime<A>) {
        let rt = unsafe { &*rt };
        self.heap_size
            .collect_at(rt.num_non_zero_events_queued() as f64, rt.sim_time());
    }
}
