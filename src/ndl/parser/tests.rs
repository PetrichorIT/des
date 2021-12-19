#[test]
fn t() {
    use crate::ndl::lexer::Token;
    use crate::ndl::souce::SourceAsset;
    use crate::ndl::{lexer::tokenize, parser::parse};
    use std::collections::VecDeque;

    let asset = SourceAsset::load(
        "./src/ndl/examples/NetworkStack.ndl".into(),
        "NetworkStack".into(),
    )
    .expect("Failed to load asset");

    let tokens = tokenize(&asset.data);
    let tokens = tokens.filter(|t| t.kind.valid());
    let tokens = tokens.filter(|t| !t.kind.reducable());
    let tokens = tokens.collect::<VecDeque<Token>>();

    let result = parse(&asset, tokens);

    if result.errors.is_empty() {
        println!("{}", result);
    } else {
        for e in result.errors {
            e.print().expect("Failed stderr");
        }
    }
}
