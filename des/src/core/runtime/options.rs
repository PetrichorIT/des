use rand::{prelude::StdRng, rngs::OsRng, SeedableRng};

use crate::SimTime;

///
/// Options for specifing the behaviour of the core runtime
/// independent of the app logic.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptions {
    ///
    /// The random number generator used internally.
    /// This can be seeded to ensure reproducability.
    /// Defaults to a [OsRng] which does NOT provide reproducability.
    ///
    pub rng: StdRng,

    ///
    /// The maximum number of events processed by the simulation. Defaults to [usize::MAX].
    ///
    pub max_itr: usize,

    ///
    /// The simtime the simulation starts on.
    ///
    pub min_sim_time: SimTime,

    ///
    /// The maximum time the simulation should reach.
    ///
    pub max_sim_time: SimTime,

    ///
    /// The number of buckets used in the cqueue for storing events.
    ///
    #[cfg(feature = "cqueue")]
    pub cqueue_num_buckets: usize,

    ///
    /// The time interval each bucket in the cqueue manages.
    ///
    #[cfg(feature = "cqueue")]
    pub cqueue_bucket_timespan: crate::SimTime,
}

impl RuntimeOptions {
    ///
    /// Creates a seeded runtime for reproducable runs.
    ///
    pub fn seeded(state: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(state),
            ..Self::default()
        }
    }

    ///
    /// Changes the maximum iteration number of a runtime.
    ///
    #[must_use]
    pub fn max_itr(mut self, max_itr: usize) -> Self {
        self.max_itr = max_itr;
        self
    }

    ///
    /// Changes the maximum time of the runtime (default: inf).
    ///
    #[must_use]
    pub fn max_time(mut self, max_time: SimTime) -> Self {
        self.max_sim_time = max_time;
        self
    }

    ///
    /// Chnages the minimum simtime of a runtime (default: 0).
    ///
    #[must_use]
    pub fn min_time(mut self, min_time: SimTime) -> Self {
        self.min_sim_time = min_time;
        self
    }
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            rng: StdRng::from_rng(OsRng::default()).unwrap(),
            max_itr: !0,
            min_sim_time: SimTime::MIN,
            max_sim_time: SimTime::MAX,

            #[cfg(feature = "cqueue")]
            cqueue_num_buckets: 10,

            #[cfg(feature = "cqueue")]
            cqueue_bucket_timespan: crate::SimTime::from(0.2),
        }
    }
}
