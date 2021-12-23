#[test]
fn test_parser() {
    use crate::SourceMap;
    use crate::TokenStream;
    use crate::{lexer::tokenize, parser::parse, AssetDescriptor};

    let mut smap = SourceMap::new();
    let asset = smap
        .load(AssetDescriptor::new(
            "./tests/ParTest.ndl".into(),
            "ParTest".into(),
        ))
        .expect("Failed to load test asset 'ParTest.ndl'");

    let tokens = tokenize(asset.source(), 0);
    let tokens = tokens.filter(|t| t.kind.valid());
    let tokens = tokens.filter(|t| !t.kind.reducable());
    let tokens = tokens.collect::<TokenStream>();

    let result = parse(asset, tokens);

    assert!(result.errors.is_empty());

    assert_eq!(result.includes.len(), 2);
    assert_eq!(result.includes[0].path, "A");
    assert_eq!(result.includes[1].path, "std/A");

    assert_eq!(result.links.len(), 1);
    assert_eq!(result.links[0].name, "NewLink");
    assert_eq!(
        (
            result.links[0].bitrate,
            result.links[0].latency,
            result.links[0].jitter
        ),
        (300, 0.1, 0.1)
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

    assert_eq!(result.modules[1].submodules.len(), 1);
    assert_eq!(result.modules[1].submodules[0].ty, "SubM");
    assert_eq!(result.modules[1].submodules[0].descriptor, "m");

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
