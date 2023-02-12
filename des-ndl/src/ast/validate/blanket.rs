use super::*;
use crate::ast::{EntryStmt, IncludeStmt};

impl Validate for IncludeStmt {
    fn validate(&self, _: &mut LinkedList<Error>) {}
}

impl Validate for EntryStmt {
    fn validate(&self, _: &mut LinkedList<Error>) {}
}
