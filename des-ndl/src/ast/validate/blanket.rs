use super::*;
use crate::ast::{EntryStmt, IncludeStmt};

impl Validate for IncludeStmt {
    fn validate(&self, _: &mut ErrorsMut) {}
}

impl Validate for EntryStmt {
    fn validate(&self, _: &mut ErrorsMut) {}
}
