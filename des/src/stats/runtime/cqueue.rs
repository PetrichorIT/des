use crate::stats::{CompressedStdDev, StdDev};
use std::io::Write;

#[cfg(feature = "metrics-rt-full")]
use super::EventCountVec;

/// Metrics that sample the runtime
pub type RuntimeMetrics = CQueueMetrics;

/// Metrics specific to a cqueue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CQueueMetrics {
    pub(crate) zero_queue_size: StdDev,
    pub(crate) bucket_queue_size: StdDev,

    pub(crate) non_zero_event_wait_time: CompressedStdDev,

    pub(crate) zero_event_count: u64,
    pub(crate) nonzero_event_count: u64,

    #[cfg(feature = "metrics-rt-full")]
    pub(crate) event_count: EventCountVec,
}

impl CQueueMetrics {
    pub(crate) fn new() -> Self {
        Self {
            zero_queue_size: StdDev::new(),
            bucket_queue_size: StdDev::new(),

            non_zero_event_wait_time: CompressedStdDev::new(0xff_ff),

            zero_event_count: 0,
            nonzero_event_count: 0,

            #[cfg(feature = "metrics-rt-full")]
            event_count: EventCountVec::new(),
        }
    }

    pub(crate) fn write_to(&self, f: &mut impl Write) -> std::io::Result<()> {
        writeln!(f, "\tinstant_queue_size: {}", self.zero_queue_size)?;
        writeln!(f, "\tbucket_queue_size: {}", self.bucket_queue_size)?;

        writeln!(f, "\tevent_timespan: {}", self.non_zero_event_wait_time)?;

        let total = self.zero_event_count + self.nonzero_event_count;
        let perc = self.nonzero_event_count as f64 / total as f64;
        writeln!(f, "\tinstant_event_prec: {}", perc)?;

        Ok(())
    }

    #[cfg(feature = "metrics-rt-full")]
    pub(crate) fn write_event_count_to(&self, f: &mut impl Write) -> std::io::Result<()> {
        self.event_count.write_to(f)
    }

    pub(crate) fn finish(&mut self) {
        self.non_zero_event_wait_time.flush();

        #[cfg(feature = "metrics-rt-full")]
        self.event_count.finish();

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
