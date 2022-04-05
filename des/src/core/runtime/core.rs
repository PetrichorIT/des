use crate::{
    core::{event::EventId, interning::Interner, SimTime, StandardLogger},
    prelude::Mrc,
    util::SyncWrap,
};
use lazy_static::lazy_static;
use rand::{
    distributions::Standard,
    prelude::{Distribution, StdRng},
    Rng,
};
lazy_static! {
    pub(crate) static ref RTC: SyncWrap<Mrc<Option<RuntimeCore>>> = SyncWrap::new(Mrc::new(None));
}

pub(crate) fn get_rtc_ptr() -> Mrc<Option<RuntimeCore>> {
    use std::ops::Deref;

    let mrc: &Mrc<Option<RuntimeCore>> = RTC.deref();
    Mrc::clone(mrc)
}

///
/// Returns the current simulation time of the currentlly active
/// runtime session.
///
#[inline(always)]
pub fn sim_time() -> SimTime {
    (*get_rtc_ptr()).as_ref().unwrap().sim_time
}

///
/// Generates a random instance of type T with a Standard distribution.
///
pub fn rng<T>() -> T
where
    Standard: Distribution<T>,
{
    get_rtc_ptr().as_mut().unwrap().rng.gen()
}

///
/// Generates a random instance of type T with a distribution
/// of type D.
///
pub fn sample<T, D>(distr: D) -> T
where
    D: Distribution<T>,
{
    get_rtc_ptr().as_mut().unwrap().rng.sample(distr)
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
    ) -> Mrc<Option<RuntimeCore>> {
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

        *get_rtc_ptr() = Some(rtc);

        get_rtc_ptr()
    }
}
