//! A crate for parsing NDL files and workspace.
//!
//! # Use Case
//!
//! NDL (NodeDescriptionLanguage) is a language for representing module and network structures
//! of a DES simulation. This crate shall be used together with des_derive the codegen
//! unit of the "NDL compiler". The provided macros will generate code based on the given
//! module / network description in the NDL files and attach that functionality to Structs
//! that implement the Module trait (for networks thats not nessecary).
//!
//! # Usage
//!
//! In general this package should only be used in either the codegen macros of des_derive
//! or in a build script, not as standalone packet.
//!
//! Nonetheless if you want to interact with NDL directly here is a quick rundown:
//!
//! ## Steps in the compile process
//!
//! | process step |                  input                  |         output         | parallisable |
//! |:------------:|:---------------------------------------:|:----------------------:|:------------:|
//! |    lexing    |                  Asset                  |       TokenStream      |     true     |
//! |    parsing   |               TokenStream               |      ParsingResult     |     true     |
//! |  desugaring  |              ParsingResult              | DesugaredParsingResult |     false    |
//! | typechecking | DesugaredParsingResult & Global Context |    validation result   |     false    |
//! |   resolving  |                Workspace                |   Global type context  |       -      |
//!
//! ## Asset management
//!
//! Assets will be managed by loading them into the [SourceMap].
//! This can be done by passing an [AssetDescriptor] to the [SourceMap::load] function.
//! The asset will be mapped internally and loaded (if not allready done) into memory.
//! If the process succeeds an [Asset] will be returned.
//! This is a object referncing the raw buffer stored in the [SourceMap].
//!
//! ## Error management
//!
//! Depending on the process steps errors will be reported differently, but in the end
//! all errors will be stored in the [GlobalErrorContext].
//! It is possible that the process may exit prematurly if a serios error was found that
//! prevents a later process step from being executed.
//! If the resolver is run in non-silent mode errors will be printed to stderr
//! after typechecking has finished.
//!
//! ## The result of a run.
//!
//! What is returned is a [OwnedTySpecContext]. This type context contains all
//! module / network specifications without checking for name collisions.
//! From there you are free to do whatever you want with this definitions.
//!

mod error;
mod loc;
mod source;

mod desugar;
mod lexer;
mod parser;
mod resolver;
mod tycheck;

// ### Exports ###

// > Function exports
pub use desugar::desugar;
pub use lexer::tokenize;
pub use lexer::tokenize_and_validate;
pub use parser::parse;
pub use tycheck::validate;
pub use tycheck::validate_module_ty;

// > Global primitivs
pub use error::Error;
pub use error::ErrorCode;
pub use error::GlobalErrorContext;
pub use lexer::Base;
pub use lexer::LiteralKind;
pub use lexer::Token;
pub use lexer::TokenKind;
pub use lexer::TokenStream;
pub use loc::Loc;
pub use source::Asset;
pub use source::AssetDescriptor;
pub use source::SourceMap;

// > Spec Exports.
pub use desugar::{
    ChannelSpec, ChildModuleSpec, ConSpec, ConSpecNodeIdent, GateSpec, IncludeSpec, ModuleSpec,
    NetworkSpec, ParamSpec, TySpec,
};
pub use parser::GateAnnotation;

#[cfg(test)]
pub use parser::TyDef;

// > TyCtx

pub use tycheck::GlobalTySpecContext;
pub use tycheck::OwnedTySpecContext;
pub use tycheck::TySpecContext;

// > Resolver
pub use resolver::NdlResolver;
pub use resolver::NdlResolverOptions;
pub use resolver::NdlResolverState;

// > Static Result
pub type NdlResult<T> = Result<T, &'static str>;
