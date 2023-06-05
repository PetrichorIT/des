use crate::time::SimTime;
use std::{fmt::Display, mem};

///
/// A composed limit that terminates the event execution of
/// a runtime.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeLimit {
    /// A unbounded runtime. A runtime with this limit will
    /// only finish if the all events are handled and no new
    /// events have been created.
    None,

    /// A bound based on the number of executed events.
    /// A runtime with this limit will terminated prematurly after the
    /// given bound is exceeded, but will finish normally if the bound-th event
    /// is the last one.
    EventCount(usize),

    /// A bound based on the simulation time.
    /// A runtime with this bound will terminate after no events
    /// scheduled before the given simulation time are left.
    SimTime(SimTime),

    /// This bound combines two other bounds with a logical AND.
    /// This will only terminated the simulation if both given
    /// limits are fulfilled.
    CombinedAnd(Box<RuntimeLimit>, Box<RuntimeLimit>),

    /// This bound combines two other bounds with a logical OR.
    /// This will terminated the simulation if one of given
    /// limits is fulfilled.
    CombinedOr(Box<RuntimeLimit>, Box<RuntimeLimit>),
}

impl RuntimeLimit {
    pub(crate) fn applies(&self, itr_count: usize, time: SimTime) -> bool {
        match self {
            Self::None => false,

            Self::EventCount(e) => itr_count > *e,
            Self::SimTime(t) => time > *t,

            Self::CombinedAnd(lhs, rhs) => {
                lhs.applies(itr_count, time) && rhs.applies(itr_count, time)
            }
            Self::CombinedOr(lhs, rhs) => {
                lhs.applies(itr_count, time) || rhs.applies(itr_count, time)
            }
        }
    }

    pub(crate) fn add(&mut self, limit: RuntimeLimit) {
        match self {
            Self::None => *self = limit,
            _ => {
                let mut other = Self::None;
                mem::swap(&mut other, self);
                *self = Self::CombinedAnd(Box::new(other), Box::new(limit));
            }
        }
    }
}

impl Display for RuntimeLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),

            Self::EventCount(e) => write!(f, "MaxEventCount({e})"),
            Self::SimTime(t) => write!(f, "MaxSimTime({t})"),

            Self::CombinedAnd(lhs, rhs) => write!(f, "{lhs} and {rhs}"),
            Self::CombinedOr(lhs, rhs) => write!(f, "{lhs} or {rhs}"),
        }
    }
}
