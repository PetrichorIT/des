use crate::core::{*, event::{Application,EventNode}};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeLimit {
    None,

    EventCount(usize),
    SimTime(SimTime),
    
    CombinedAnd(Box<RuntimeLimit>, Box<RuntimeLimit>),
    CombinedOr(Box<RuntimeLimit>, Box<RuntimeLimit>),
}

impl RuntimeLimit {
    pub(crate) fn applies<A>(&self, itr_count: usize, node: &EventNode<A>) -> bool where A: Application {
        match self {
            Self::None => false,

            Self::EventCount(e) => itr_count > *e,
            Self::SimTime(t) => node.time > *t,

            Self::CombinedAnd(lhs, rhs) => lhs.applies(itr_count, node) && rhs.applies(itr_count, node),
            Self::CombinedOr(lhs, rhs) => lhs.applies(itr_count, node) || rhs.applies(itr_count, node),
        }
    }
}

impl Display for RuntimeLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),

            Self::EventCount(e) => write!(f, "MaxEventCount({})", e),
            Self::SimTime(t) => write!(f, "MaxSimTime({})", t),

            Self::CombinedAnd(lhs, rhs) => write!(f, "{} and {}", lhs, rhs),
            Self::CombinedOr(lhs, rhs) => write!(f, "{} or {}", lhs, rhs),
        }
    }
}
