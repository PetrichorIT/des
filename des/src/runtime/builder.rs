use std::{
    fmt::Debug,
    sync::{Mutex, TryLockError},
};

#[cfg(feature = "cqueue")]
use std::time::Duration;

use rand::{
    rngs::{OsRng, StdRng},
    RngCore, SeedableRng,
};

use crate::prelude::SimTime;

use super::{Application, FutureEventSet, Profiler, Runtime, RuntimeLimit, RNG};

/// A lock the ensures only one runtime exits at a time.
static SIMULATION_LOCK: Mutex<()> = Mutex::new(());

/// A builder for a runtime instance.
#[must_use]
pub struct Builder {
    pub(super) quiet: bool,
    pub(super) rng: Box<dyn RngCore>,
    pub(super) limit: RuntimeLimit,
    pub(super) start_time: SimTime,

    #[cfg(feature = "cqueue")]
    pub(super) cqueue_num_buckets: usize,
    #[cfg(feature = "cqueue")]
    pub(super) cqueue_bucket_timespan: Duration,
}

impl Builder {
    /// Creates a new unconfigured builder.
    ///
    /// # Panics
    ///
    /// Panics if no RNG can be build.
    pub fn new() -> Builder {
        Builder {
            quiet: false,
            rng: Box::new(StdRng::from_rng(OsRng).expect("Failed to create RNG")),
            limit: RuntimeLimit::None,

            start_time: SimTime::MIN,

            #[cfg(feature = "cqueue")]
            cqueue_num_buckets: 1028,

            #[cfg(feature = "cqueue")]
            cqueue_bucket_timespan: Duration::from_secs_f64(0.0025),
        }
    }

    /// Creates a `Builder` with a static seeded RNG.
    pub fn seeded(seed: u64) -> Builder {
        Builder {
            quiet: false,
            rng: Box::new(StdRng::seed_from_u64(seed)),
            limit: RuntimeLimit::None,

            start_time: SimTime::MIN,

            #[cfg(feature = "cqueue")]
            cqueue_num_buckets: 1028,

            #[cfg(feature = "cqueue")]
            cqueue_bucket_timespan: Duration::from_secs_f64(0.0025),
        }
    }

    ///
    /// Sets the cqueue options if this runtime uses a cqueue.
    /// NOP otherwise.
    ///
    #[cfg(feature = "cqueue")]
    pub fn cqueue_options(mut self, n: usize, t: Duration) -> Self {
        self.cqueue_num_buckets = n;
        self.cqueue_bucket_timespan = t;

        self
    }

    ///
    /// Suppressed runtime messages from the simulation framework.
    ///
    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    ///
    /// Changes the maximum iteration number of a runtime.
    ///
    pub fn start_time(mut self, time: SimTime) -> Self {
        self.start_time = time;
        self
    }

    ///
    /// Changes the maximum iteration number of a runtime.
    ///
    pub fn max_itr(mut self, max_itr: usize) -> Self {
        self.limit.add(RuntimeLimit::EventCount(max_itr));
        self
    }

    ///
    /// Changes the maximum time of the runtime (default: inf).
    ///
    pub fn max_time(mut self, max_time: SimTime) -> Self {
        self.limit.add(RuntimeLimit::SimTime(max_time));
        self
    }

    ///
    /// Sets a custom limit to the end of the runtime, overwriting
    /// all `max_itr` and `max_time` options.
    ///
    pub fn limit(mut self, limit: RuntimeLimit) -> Self {
        self.limit.add(limit);
        self
    }

    ///
    /// Builds a new [`Runtime`] instance, using an application as core,
    /// and accepting events of type [`Event<A>`](crate::runtime::Event).
    ///
    /// # Examples
    ///
    /// ```
    /// use des::prelude::*;
    ///
    /// // Assumme Application is implemented for App.
    /// #[derive(Debug)]
    /// struct App(usize,  String);
    /// # impl Application for App {
    /// #   type EventSet = Events;
    /// #   type Lifecycle = ();
    /// # }
    /// # enum Events {}
    /// # impl EventSet<App> for Events {
    /// #   fn handle(self, rt: &mut Runtime<App>) {}
    /// # }
    ///
    /// let app = App(42, String::from("Hello there!"));
    /// let rt = Builder::new().build(app);
    /// ```
    pub fn build<A: Application>(self, app: A) -> Runtime<A> {
        let permit = {
            let lock = SIMULATION_LOCK.try_lock();
            match lock {
                Ok(permit) => permit,
                Err(err) => {
                    match err {
                        TryLockError::WouldBlock => {
                            eprintln!("des::warning ** another runtime allready exists ... waiting for simlock");
                            let lock = SIMULATION_LOCK.lock();
                            match lock {
                                Ok(lock) => lock,
                                Err(p) => {
                                    eprintln!("des::error ** another runtime poisoned the simlock ... cleaning up");
                                    Runtime::<A>::poison_cleanup();
                                    p.into_inner()
                                }
                            }
                        }
                        TryLockError::Poisoned(p) => {
                            eprintln!("des::error ** another runtime poisoned the simlock ... cleaning up");
                            Runtime::<A>::poison_cleanup();
                            p.into_inner()
                        }
                    }
                }
            }
        };

        // Log prep
        // StandardLogger::setup().expect("Failed to create logger");
        #[cfg(feature = "cqueue")]
        if std::mem::size_of::<A::EventSet>() > 128 {
            eprintln!("des::warning ** creating runtime with event-set bigger that 128 bytes * this may lead to performance losses");
        }

        let future_event_set = FutureEventSet::new_with(&self);

        // Set SimTime
        SimTime::set_now(self.start_time);

        // Set RNG
        *unsafe { &mut *RNG.get() } = Some(self.rng);

        Runtime {
            future_event_set,

            event_id: 0,
            itr: 0,
            permit,

            limit: self.limit,

            quiet: self.quiet,
            profiler: Profiler::default(),

            app,
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Builder::new()
    }
}

impl Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder").finish()
    }
}
