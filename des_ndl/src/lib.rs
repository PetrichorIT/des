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

// > Global primitivs
pub use error::Error;
pub use error::ErrorCode;
pub use error::GlobalErrorContext;
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
    NetworkSpec, ParamSpec,
};

// > TyCtx
pub use desugar::GlobalTyDefContext;
pub use desugar::TyDefContext;
pub use tycheck::GlobalTySpecContext;
pub use tycheck::OwnedTySpecContext;
pub use tycheck::TySpecContext;

// > Resolver
pub use resolver::NdlResolver;
pub use resolver::NdlResolverOptions;
pub use resolver::NdlResolverState;

// > Static Result
pub type ParResult<T> = Result<T, &'static str>;

mod tests {
    #[test]
    fn mtest() {
        use crate::*;

        let mut resolver = NdlResolver::new("./examples")
            .expect("Failed to create test resolver from examples directory");

        let _ = resolver.run();
        println!("{}", resolver.units.get("Main").unwrap());
        panic!("WOLOLO");
    }
}
