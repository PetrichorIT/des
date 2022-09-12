use std::io::Write;

cfg_not_cqueue! {
    /// Metrics that sample the runtime
    pub type RuntimeMetrics = OptimizedBinaryHeapMetrics;

    use crate::stats::CompressedStdDev;
    use crate::stats::StdDev;

    /// Metrics specific to a binary heap.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OptimizedBinaryHeapMetrics {
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

        pub(crate) fn write_to(&self, f: &mut impl Write) -> std::io::Result<()> {
            writeln!(f, "\theap_size: {}", self.heap_size)?;
            writeln!(f,
                "\tevent_timespan: {}",
                self.non_zero_event_wait_time
            )?;
            writeln!(f, "\tinstant_event_prec: {}", self.zero_event_prec)?;

            let total = self.zero_event_count + self.non_zero_event_count;
            let perc = self.non_zero_event_count as f64 / total as f64;
            writeln!(f, "\ttinstant_event_prec: {}", perc)?;

            Ok(())
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

cfg_cqueue! {
    /// Metrics that sample the runtime
    pub type RuntimeMetrics = CQueueMetrics;

    use crate::stats::{CompressedStdDev, StdDev, MeanVec};

    /// Metrics specific to a cqueue.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CQueueMetrics {
        pub(crate) event_count: MeanVec,

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

        pub(crate) fn write_to(&self, f: &mut impl Write) -> std::io::Result<()> {

            writeln!(f, "\tinstant_queue_size: {}", self.zero_queue_size)?;
            writeln!(f, "\tbucket_queue_size: {}", self.bucket_queue_size)?;

            writeln!(f,
                "\tevent_timespan: {}",
                self.non_zero_event_wait_time
            )?;

            let total = self.zero_event_count + self.nonzero_event_count;
            let perc = self.nonzero_event_count as f64 / total as f64;
            writeln!(f, "\tinstant_event_prec: {}", perc)?;

            Ok(())
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
