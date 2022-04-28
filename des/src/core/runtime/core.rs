use crate::{
    core::{event::EventId, runtime::RuntimeLimit, SimTime, StandardLogger},
    util::{PtrMut, SyncWrap},
};
use lazy_static::lazy_static;
use rand::{
    distributions::Standard,
    prelude::{Distribution, StdRng},
    Rng,
};
lazy_static! {
    pub(crate) static ref RTC: SyncWrap<PtrMut<Option<RuntimeCore>>> =
        SyncWrap::new(PtrMut::new(None));
}

pub(crate) fn get_rtc_ptr() -> PtrMut<Option<RuntimeCore>> {
    use std::ops::Deref;

    let ptr: &PtrMut<Option<RuntimeCore>> = RTC.deref();
    PtrMut::clone(ptr)
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

    // Rt limits
    pub limit: RuntimeLimit,

    pub event_id: EventId,
    pub itr: usize,

    // Misc
    pub rng: StdRng,
}

impl RuntimeCore {
    pub fn new(
        sim_time: SimTime,
        event_id: EventId,
        itr: usize,
        limit: RuntimeLimit,
        rng: StdRng,
    ) -> PtrMut<Option<RuntimeCore>> {
        let rtc = Self {
            sim_time,
            
            limit,
            event_id,
            itr,

            rng,
        };

        if let Err(e) = StandardLogger::setup() {
            eprintln!("{}", e)
        }

        *get_rtc_ptr() = Some(rtc);

        get_rtc_ptr()
    }
}
