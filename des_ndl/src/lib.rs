mod error;
mod loc;
mod source;

pub use error::{Error, ErrorCode, GlobalErrorContext, ParsingErrorContext};
pub use loc::Loc;
pub use source::{SourceAsset, SourceAssetDescriptor};

mod lexer;
mod parser;
mod resolver;
mod tycheck;

pub use lexer::{tokenize, tokenize_and_validate, Token, TokenKind, TokenStream};
pub use parser::{
    parse, ConDef, ConNodeIdent, GateDef, LinkDef, ModuleDef, NetworkDef, ParsingResult,
};
pub use resolver::NdlResolver;
pub use tycheck::{validate, TyContext};

mod tests {
    #[test]
    fn mtest() {
        use crate::*;

        let mut resolver = NdlResolver::new("./examples")
            .expect("Failed to create test resolver from examples directory");

        resolver.run();
        // println!("{}", resolver);
        panic!("WOLOLO");
    }
}
