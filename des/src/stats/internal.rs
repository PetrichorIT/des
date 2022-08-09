cfg_not_cqueue! {
    /// Metrics that sample the runtime
    pub(crate) type RuntimeMetrics = std::OptimizedBinaryHeapMetrics;

    mod std {
        use crate::stats::CompressedStdDev;
        use crate::stats::StdDev;

        #[derive(Debug)]
        pub(crate) struct OptimizedBinaryHeapMetrics {
            pub(crate) heap_size: StdDev,

            pub(crate) non_zero_event_wait_time: CompressedStdDev,
            pub(crate) zero_event_prec: CompressedStdDev,

            pub(crate) zero_event_count: u64,
            pub(crate) non_zero_event_count: u64,
        }

        impl OptimizedBinaryHeapMetrics {
            pub(crate) fn new() -> Self {
                Self {
                    heap_size: StdDev::new(),

                    non_zero_event_wait_time: CompressedStdDev::new(0xff_ff),
                    zero_event_prec: CompressedStdDev::new(0xff_ff),

                    zero_event_count: 0,
                    non_zero_event_count: 0,
                }
            }

            pub(crate) fn finish(&mut self) {
                self.non_zero_event_wait_time.flush();
                self.zero_event_prec.flush();

                println!("\u{23A2} Metrics");

                println!("\u{23A2}  Heap size:          {}", self.heap_size);
                println!(
                    "\u{23A2}  Event timespan:     {}",
                    self.non_zero_event_wait_time
                );
                println!("\u{23A2}  Instant event prec: {}", self.zero_event_prec);

                let total = self.zero_event_count + self.non_zero_event_count;
                let perc = self.non_zero_event_count as f64 / total as f64;
                println!("\u{23A2}  Instant event prec: {}", perc);
            }
        }
    }
}

cfg_cqueue! {
    /// Metrics that sample the runtime
    pub(crate) type RuntimeMetrics = CQueueMetrics;

    use crate::stats::{CompressedStdDev, StdDev};

#[derive(Debug)]
pub(crate) struct CQueueMetrics {
    pub(crate) zero_queue_size: StdDev,
    pub(crate) bucket_queue_size: StdDev,

    pub(crate) non_zero_event_wait_time: CompressedStdDev,

    pub(crate) zero_event_count: u64,
    pub(crate) nonzero_event_count: u64,
}

impl CQueueMetrics {
    pub(crate) fn new() -> Self {
        Self {
            zero_queue_size: StdDev::new(),
            bucket_queue_size: StdDev::new(),

            non_zero_event_wait_time: CompressedStdDev::new(0xff_ff),

            zero_event_count: 0,
            nonzero_event_count: 0,
        }
    }

    pub(crate) fn finish(&mut self) {
        self.non_zero_event_wait_time.flush();

        println!("\u{23A2} Metrics");

        println!("\u{23A2}  Instant-queue size: {}", self.zero_queue_size);
        println!("\u{23A2}  Bucket-queue size:  {}", self.bucket_queue_size);

        println!(
            "\u{23A2}  Event timespan:     {}",
            self.non_zero_event_wait_time
        );

        let total = self.zero_event_count + self.nonzero_event_count;
        let perc = self.nonzero_event_count as f64 / total as f64;
        println!("\u{23A2}  Instant event prec: {}", perc);
    }
}
}