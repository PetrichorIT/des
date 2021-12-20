#[test]
fn test_parser() {
    use des_core::ChannelMetrics;
    use std::collections::VecDeque;

    use crate::lexer::Token;
    use crate::source::SourceAsset;
    use crate::{lexer::tokenize, parser::parse, SourceAssetDescriptor};

    let asset = SourceAsset::load(SourceAssetDescriptor::new(
        "./tests/ParTest.ndl".into(),
        "ParTest".into(),
    ))
    .expect("Failed to load test asset 'ParTest.ndl'");

    println!("{}", asset.lines);

    let tokens = tokenize(&asset.data);
    let tokens = tokens.filter(|t| t.kind.valid());
    let tokens = tokens.filter(|t| !t.kind.reducable());
    let tokens = tokens.collect::<VecDeque<Token>>();

    let result = parse(&asset, tokens);

    assert!(result.errors.is_empty());

    assert_eq!(result.includes.len(), 2);
    assert_eq!(result.includes[0].path, "A");
    assert_eq!(result.includes[1].path, "std/A");

    assert_eq!(result.links.len(), 1);
    assert_eq!(result.links[0].name, "NewLink");
    assert_eq!(
        result.links[0].metrics,
        ChannelMetrics::new(300, 0.1.into(), 0.1.into())
    );

    assert_eq!(result.modules.len(), 2);

    assert_eq!(result.modules[0].name, "SubM");
    assert_eq!(result.modules[0].gates.len(), 1);
    assert_eq!(result.modules[0].gates[0].name, "another");
    assert_eq!(result.modules[0].gates[0].size, 5);

    assert_eq!(result.modules[0].parameters.len(), 1);
    assert_eq!(result.modules[0].parameters[0].ident, "addr");
    assert_eq!(result.modules[0].parameters[0].ty, "usize");

    assert_eq!(result.modules[1].name, "Main");
    assert_eq!(result.modules[1].gates.len(), 3);
    assert_eq!(result.modules[1].gates[0].name, "some");
    assert_eq!(result.modules[1].gates[0].size, 5);
    assert_eq!(result.modules[1].gates[1].name, "same");
    assert_eq!(result.modules[1].gates[1].size, 5);
    assert_eq!(result.modules[1].gates[2].name, "sike");
    assert_eq!(result.modules[1].gates[2].size, 1);

    assert_eq!(result.modules[1].submodule.len(), 1);
    assert_eq!(result.modules[1].submodule[0].ty, "SubM");
    assert_eq!(result.modules[1].submodule[0].descriptor, "m");

    assert_eq!(result.modules[1].connections.len(), 2);
    assert_eq!(result.modules[1].connections[0].channel, None);
    assert_eq!(result.modules[1].connections[0].from.ident, "some");
    assert_eq!(result.modules[1].connections[0].to.ident, "m");
    assert_eq!(
        result.modules[1].connections[0].to.subident,
        Some("another".into())
    );
    assert_eq!(
        result.modules[1].connections[1].channel,
        Some("NewLink".into())
    );
    assert_eq!(result.modules[1].connections[1].from.ident, "sike");
    assert_eq!(result.modules[1].connections[1].to.ident, "same");
}
