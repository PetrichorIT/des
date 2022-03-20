use rand::{prelude::StdRng, rngs::OsRng, SeedableRng};

///
/// Options for sepcificing the behaviour of the core runtime
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
    pub fn seeded(state: u64) -> Self {
        let mut default = Self::default();
        default.rng = StdRng::seed_from_u64(state);
        default
    }

    pub fn max_itr(mut self, max_itr: usize) -> Self {
        self.max_itr = max_itr;
        self
    }
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            rng: StdRng::from_rng(OsRng::default()).unwrap(),
            max_itr: !0,

            #[cfg(feature = "cqueue")]
            cqueue_num_buckets: 10,

            #[cfg(feature = "cqueue")]
            cqueue_bucket_timespan: crate::SimTime::from(0.2),
        }
    }
}
