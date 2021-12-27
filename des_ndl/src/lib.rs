mod error;
mod loc;
mod source;

pub use error::{Error, ErrorCode, GlobalErrorContext, ParsingErrorContext};
pub use loc::Loc;
pub use source::{AssetDescriptor, SourceMap};

mod desugar;
mod lexer;
mod parser;
mod resolver;
mod tycheck;

pub use desugar::*;
pub use lexer::{tokenize, tokenize_and_validate, Token, TokenKind, TokenStream};
pub use parser::{
    parse, ChildeModuleDef, ConDef, ConNodeIdent, GateDef, LinkDef, LocalDescriptorDef, ModuleDef,
    NetworkDef, ParamDef, ParsingResult,
};
pub use resolver::{NdlResolver, NdlResolverOptions};
pub use tycheck::{validate, GlobalTySpecContext, OwnedTySpecContext, TySpecContext};

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
