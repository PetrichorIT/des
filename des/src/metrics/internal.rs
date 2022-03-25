#[cfg(not(feature = "cqueue"))]
pub type RuntimeMetrics = std::OptimizedBinaryHeapMetrics;

#[cfg(not(feature = "cqueue"))]
mod std {
    use crate::metrics::CompressedStdDev;
    use crate::metrics::StdDev;

    pub struct OptimizedBinaryHeapMetrics {
        pub heap_size: StdDev,

        pub non_zero_event_wait_time: CompressedStdDev,
        pub zero_event_prec: CompressedStdDev,

        pub zero_event_count: u64,
        pub non_zero_event_count: u64,
    }

    impl OptimizedBinaryHeapMetrics {
        pub fn new() -> Self {
            Self {
                heap_size: StdDev::new(),

                non_zero_event_wait_time: CompressedStdDev::new(0xff_ff),
                zero_event_prec: CompressedStdDev::new(0xff_ff),

                zero_event_count: 0,
                non_zero_event_count: 0,
            }
        }

        pub fn finish(&mut self) {
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

#[cfg(feature = "cqueue")]
pub type RuntimeMetrics = cqueue::CQueueMetrics;

#[cfg(feature = "cqueue")]
mod cqueue {
    use crate::metrics::{CompressedStdDev, StdDev};

    pub struct CQueueMetrics {
        pub overflow_heap_size: StdDev,
        pub queue_bucket_size: StdDev,
        pub avg_first_bucket_fill: StdDev,
        pub avg_filled_buckets: StdDev,

        pub non_zero_event_wait_time: CompressedStdDev,

        pub zero_event_count: u64,
        pub nonzero_event_count: u64,
    }

    impl CQueueMetrics {
        pub fn new() -> Self {
            Self {
                overflow_heap_size: StdDev::new(),
                queue_bucket_size: StdDev::new(),
                avg_first_bucket_fill: StdDev::new(),
                avg_filled_buckets: StdDev::new(),

                non_zero_event_wait_time: CompressedStdDev::new(0xff_ff),

                zero_event_count: 0,
                nonzero_event_count: 0,
            }
        }

        pub fn finish(&mut self) {
            self.non_zero_event_wait_time.flush();

            println!("\u{23A2} Metrics");

            println!("\u{23A2}  Bucket queue total: {}", self.queue_bucket_size);
            println!(
                "\u{23A2}  Per bucket total:   {}",
                self.avg_first_bucket_fill
            );
            println!("\u{23A2}  Num filled buckets: {}", self.avg_filled_buckets);

            println!("\u{23A2}  Overflow Heap size: {}", self.overflow_heap_size);
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
