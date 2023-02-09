pub(crate) mod ast;
pub(crate) mod context;
pub(crate) mod error;
pub(crate) mod ir;
pub(crate) mod lexer;
pub(crate) mod resolve;

#[allow(unused)]
pub(crate) mod resource;

pub use self::resource::Asset;
pub use self::resource::SourceMap;
pub use self::resource::Span;

pub use self::error::Error;
pub use self::error::ErrorHint;
pub use self::error::ErrorKind;
pub use self::error::ErrorSolution;
pub use self::error::Result;

pub use self::ast::expr::*;
pub use self::ast::parse::*;
pub use self::ast::token::*;

pub use self::context::Context;
pub use self::ir::*;
pub use self::resolve::*;
