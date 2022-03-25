use crate::{
    core::{event::EventId, interning::Interner, SimTime, StandardLogger},
    util::SyncCell,
};
use lazy_static::lazy_static;
use rand::{
    distributions::Standard,
    prelude::{Distribution, StdRng},
    Rng,
};

lazy_static! {
    pub(crate) static ref RTC: SyncCell<Option<RuntimeCore>> = SyncCell::new(None);
}

///
/// Returns the current simulation time of the currentlly active
/// runtime session.
///
#[inline(always)]
pub fn sim_time() -> SimTime {
    unsafe { (*RTC.get()).as_ref().unwrap().sim_time }
}

///
/// Generates a random instance of type T with a Standard distribution.
///
pub fn rng<T>() -> T
where
    Standard: Distribution<T>,
{
    unsafe { (*RTC.get()).as_mut().unwrap().rng.gen() }
}

///
/// Generates a random instance of type T with a distribution
/// of type D.
///
pub fn sample<T, D>(distr: D) -> T
where
    D: Distribution<T>,
{
    unsafe { (*RTC.get()).as_mut().unwrap().rng.sample(distr) }
}

#[derive(Debug)]
pub(crate) struct RuntimeCore {
    pub sim_time: SimTime,
    pub max_sim_time: SimTime,

    // Rt limits
    pub event_id: EventId,
    pub itr: usize,
    pub max_itr: usize,

    // interning
    pub interner: Interner,

    // Misc
    pub rng: StdRng,
}

impl RuntimeCore {
    pub fn new(
        sim_time: SimTime,
        event_id: EventId,
        itr: usize,
        max_itr: usize,
        max_sim_time: SimTime,
        rng: StdRng,
    ) -> &'static SyncCell<Option<RuntimeCore>> {
        let rtc = Self {
            sim_time,
            max_sim_time,

            event_id,
            itr,
            max_itr,

            interner: Interner::new(),

            rng,
        };

        if let Err(e) = StandardLogger::setup() {
            eprintln!("{}", e)
        }

        unsafe { *RTC.get() = Some(rtc) };

        &RTC
    }
}
