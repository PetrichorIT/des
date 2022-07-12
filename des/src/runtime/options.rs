use crate::runtime::*;
use rand::{prelude::StdRng, SeedableRng};

///
/// Options for specifing the behaviour of the core runtime
/// independent of the app logic.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    ///
    /// Whether the simulation should output any values to stdout.
    ///
    pub quiet: bool,

    ///
    /// The random number generator used internally.
    /// This can be seeded to ensure reproducability.
    /// Defaults to a [OsRng] which does NOT provide reproducability.
    ///
    pub rng: Option<StdRng>,

    ///
    /// The maximum number of events processed by the simulation. Defaults to [usize::MAX].
    ///
    pub max_itr: Option<usize>,

    ///
    /// The simtime the simulation starts on.
    ///
    pub min_sim_time: Option<SimTime>,

    ///
    /// A more complexe custom limit that determines the end of the simulation
    /// overwriting 'max_itr' and 'max_sim_time' if set.
    ///
    pub custom_limit: Option<RuntimeLimit>,

    ///
    /// The maximum time the simulation should reach.
    ///
    pub max_sim_time: Option<SimTime>,

    ///
    /// The number of buckets used in the cqueue for storing events.
    ///
    #[cfg(feature = "cqueue")]
    pub cqueue_num_buckets: usize,

    ///
    /// The time interval each bucket in the cqueue manages.
    ///
    #[cfg(feature = "cqueue")]
    pub cqueue_bucket_timespan: crate::time::Duration,
}

impl RuntimeOptions {
    ///
    /// Creates a seeded runtime for reproducable runs.
    ///
    pub fn seeded(state: u64) -> Self {
        Self {
            rng: Some(StdRng::seed_from_u64(state)),
            ..Self::default()
        }
    }

    ///
    /// Sets the cqueue options if this runtime uses a cqueue.
    /// NOP otherwise.
    ///
    #[allow(unused)]
    #[must_use]
    pub fn cqueue_options(mut self, n: usize, t: Duration) -> Self {
        #[cfg(feature = "cqueue")]
        {
            self.cqueue_num_buckets = n;
            self.cqueue_bucket_timespan = t;
        }
        self
    }

    ///
    /// Suppressed runtime messages from the simulation framework.
    ///
    #[must_use]
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    ///
    /// Changes the maximum iteration number of a runtime.
    ///
    #[must_use]
    pub fn max_itr(mut self, max_itr: usize) -> Self {
        self.max_itr = Some(max_itr);
        self
    }

    ///
    /// Changes the maximum time of the runtime (default: inf).
    ///
    #[must_use]
    pub fn max_time(mut self, max_time: SimTime) -> Self {
        self.max_sim_time = Some(max_time);
        self
    }

    ///
    /// Changes the minimum simtime of a runtime (default: 0).
    ///
    #[must_use]
    pub fn min_time(mut self, min_time: SimTime) -> Self {
        self.min_sim_time = Some(min_time);
        self
    }

    ///
    /// Sets a custom limit to the end of the runtime, overwriting
    /// all max_itr and max_time options.
    ///
    #[must_use]
    pub fn limit(mut self, limit: RuntimeLimit) -> Self {
        self.custom_limit = Some(limit);
        self
    }
}

// PLEASE make clippy consider #[cfg(feature)]
#[allow(clippy::derivable_impls)]
impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            quiet: false,

            rng: None,
            max_itr: None,
            min_sim_time: None,
            max_sim_time: None,

            custom_limit: None,

            #[cfg(feature = "cqueue")]
            cqueue_num_buckets: 1028,

            #[cfg(feature = "cqueue")]
            cqueue_bucket_timespan: crate::time::Duration::from_secs_f64(0.0025),
        }
    }
}
