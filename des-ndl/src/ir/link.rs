use crate::ast::{self, LinkStmt, Spanned};
use std::{collections::HashMap, fmt, sync::Arc};

use super::*;

#[derive(Clone, PartialEq)]
pub struct Link {
    pub ast: Arc<LinkStmt>,

    pub ident: RawSymbol,
    pub fields: HashMap<String, Literal>,

    // common
    pub jitter: f64,
    pub latency: f64,
    pub bitrate: i32,

    pub(crate) dirty: bool,
}

impl fmt::Debug for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Link")
            .field("ast", &self.ast.span())
            .field("ident", &self.ident)
            .field("fields", &self.fields)
            .field("jitter", &self.jitter)
            .field("latency", &self.latency)
            .field("bitrate", &self.bitrate)
            .field("dirty", &self.dirty)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Float(f64),
    Integer(i32),
    String(String),
}

impl Literal {
    pub fn as_float_casted(&self) -> f64 {
        match self {
            Self::Float(f) => *f,
            Self::Integer(i) => *i as f64,
            Self::String(s) => s.parse().expect("Force casted as float"),
        }
    }

    pub fn as_integer_casted(&self) -> i32 {
        match self {
            Self::Integer(i) => *i,
            Self::Float(f) => *f as i32,
            Self::String(s) => s.parse().expect("Force casted as integer"),
        }
    }
}

impl From<ast::Lit> for Literal {
    fn from(value: ast::Lit) -> Self {
        match value.kind {
            ast::LitKind::Float { lit } => Literal::Float(lit),
            ast::LitKind::Integer { lit } => Literal::Integer(lit),
            ast::LitKind::Str { lit } => Literal::String(lit),
        }
    }
}
