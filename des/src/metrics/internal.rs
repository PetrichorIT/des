use std::ops::Sub;

use crate::core::{Application, Runtime, SimTime};
use crate::metrics::CompressedStdDev;
use crate::metrics::StdDev;
use crate::Statistic;

pub struct RuntimeMetrics {
    pub heap_size: StdDev,

    pub non_zero_event_wait_time: CompressedStdDev,
    pub zero_event_prec: CompressedStdDev,

    pub zero_event_count: u64,
    pub non_zero_event_count: u64,
}

impl RuntimeMetrics {
    pub fn new() -> Self {
        Self {
            heap_size: StdDev::new(),

            non_zero_event_wait_time: CompressedStdDev::new(0xff_ff),
            zero_event_prec: CompressedStdDev::new(0xff_ff),

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

        let total = rt.num_zero_events_queued() + rt.num_non_zero_events_queued() + 1;
        let perc = (rt.num_zero_events_queued() as f64) / (total as f64);
        self.zero_event_prec.collect_at(perc, rt.sim_time())
    }

    pub fn finish(&mut self) {
        self.non_zero_event_wait_time.flush();
        self.zero_event_prec.flush();

        println!("Metrics");
        println!("=======");

        println!("Heap size:          {}", self.heap_size);
        println!("Event timespan:     {}", self.non_zero_event_wait_time);
        println!("Instant event prec: {}", self.zero_event_prec);

        let total = self.zero_event_count + self.non_zero_event_count;
        let perc = self.non_zero_event_count as f64 / total as f64;
        println!("Instant event prec: {}", perc);
    }
}
