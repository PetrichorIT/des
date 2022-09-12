use crate::stats::MeanVec;
use std::{
    io::{Result, Write},
    ops::{Deref, DerefMut},
    time::Duration,
};

macro_rules! nnan {
    ($e: expr) => {
        if $e.is_nan() {
            0.0
        } else {
            *$e
        }
    };
}

pub(crate) const EVENT_COUNT_VEC_SLOT_SIZE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EventCountVec {
    pub(super) inner: MeanVec,
}

impl EventCountVec {
    pub(crate) fn new() -> Self {
        Self {
            inner: MeanVec::new(EVENT_COUNT_VEC_SLOT_SIZE),
        }
    }

    pub(crate) fn write_to(&self, f: &mut impl Write) -> Result<()> {
        write!(f, "{{ \"results\": [ ")?;
        for (i, (time, mean, min, max, stddev)) in self.inner.results.iter().enumerate() {
            if i == self.inner.results.len() - 1 {
                write!(f, "{{ \"command\": \"mean-vec\", \"mean\": {}, \"min\": {}, \"max\": {}, \"stddev\": {}, \"median\": {}, \"parameters\": {{ \"time\": {} }} }}", nnan!(mean), nnan!(min), nnan!(max), nnan!(stddev), nnan!(mean), time.as_secs_f64().ceil())?
            } else {
                write!(f, "{{ \"command\": \"mean-vec\", \"mean\": {}, \"min\": {}, \"max\": {}, \"stddev\": {}, \"median\": {}, \"parameters\": {{ \"time\": {} }} }},", nnan!(mean), nnan!(min), nnan!(max), nnan!(stddev), nnan!(mean), time.as_secs_f64().ceil())?
            }
        }
        write!(f, "] }}")?;

        Ok(())
    }
}

impl Deref for EventCountVec {
    type Target = MeanVec;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for EventCountVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
