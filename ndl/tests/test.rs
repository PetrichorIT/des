use ndl::*;

#[test]
#[cfg(target_os = "linux")]
fn lex_full() {
    let contents = std::fs::read_to_string("tests/LexTest.ndl")
        .expect("Failed to read static test file 'LexTest.ndl'");

    let token_stream = tokenize(&contents, 0).collect::<Vec<Token>>();

    let zip = token_stream
        .iter()
        .zip(vec![
            Token::new(TokenKind::Ident, 0, 6, 1),
            Token::new(TokenKind::Whitespace, 6, 1, 1),
            Token::new(TokenKind::And, 7, 1, 1),
            Token::new(TokenKind::Ident, 8, 1, 1),
            Token::new(TokenKind::Whitespace, 9, 1, 1),
            Token::new(TokenKind::OpenBrace, 10, 1, 1),
            Token::new(TokenKind::Whitespace, 11, 5, 1),
            Token::new(TokenKind::Plus, 16, 1, 2),
            Token::new(TokenKind::Minus, 17, 1, 2),
            Token::new(TokenKind::Whitespace, 18, 1, 2),
            Token::new(TokenKind::Colon, 19, 1, 2),
            Token::new(TokenKind::Whitespace, 20, 1, 2),
            Token::new(
                TokenKind::Literal {
                    kind: LiteralKind::Int {
                        base: Base::Decimal,
                        empty_int: false,
                    },
                    suffix_start: 6,
                },
                21,
                6,
                2,
            ),
            Token::new(TokenKind::Whitespace, 27, 1, 2),
            Token::new(TokenKind::CloseBrace, 28, 1, 3),
        ])
        .enumerate();

    for (i, (lhs, rhs)) in zip {
        assert_eq!(*lhs, rhs, "Token #{} missmatched", i)
    }

    let token_stream = token_stream
        .into_iter()
        .filter(|t| t.kind.valid() && !t.kind.reducable())
        .collect::<Vec<Token>>();

    let zip = token_stream
        .iter()
        .zip(vec![
            Token::new(TokenKind::Ident, 0, 6, 1),
            Token::new(TokenKind::Whitespace, 6, 1, 1),
            Token::new(TokenKind::Ident, 8, 1, 1),
            Token::new(TokenKind::Whitespace, 9, 1, 1),
            Token::new(TokenKind::OpenBrace, 10, 1, 1),
            Token::new(TokenKind::Whitespace, 11, 5, 1),
            Token::new(TokenKind::Minus, 17, 1, 2),
            Token::new(TokenKind::Whitespace, 18, 1, 2),
            Token::new(TokenKind::Colon, 19, 1, 2),
            Token::new(TokenKind::Whitespace, 20, 1, 2),
            Token::new(
                TokenKind::Literal {
                    kind: LiteralKind::Int {
                        base: Base::Decimal,
                        empty_int: false,
                    },
                    suffix_start: 6,
                },
                21,
                6,
                2,
            ),
            Token::new(TokenKind::Whitespace, 27, 1, 2),
            Token::new(TokenKind::CloseBrace, 28, 1, 3),
        ])
        .enumerate();

    for (i, (lhs, rhs)) in zip {
        assert_eq!(*lhs, rhs, "Token #{} missmatched", i)
    }
}

#[test]
fn ndl_parser_test() {
    let mut smap = SourceMap::new();
    let asset = smap
        .load(AssetDescriptor::new(
            "tests/ParTest.ndl".into(),
            "ParTest".into(),
        ))
        .expect("Failed to load test asset 'ParTest.ndl'");

    let tokens = tokenize(asset.source(), 0);
    let tokens = tokens.filter(|t| t.kind.valid());
    let tokens = tokens.filter(|t| !t.kind.reducable());
    let tokens = tokens.collect::<TokenStream>();

    let result = parse(asset, tokens);

    for error in &result.errors {
        error.print(&smap).unwrap();
    }

    assert!(result.errors.is_empty());

    assert_eq!(result.includes.len(), 2);
    assert_eq!(result.includes[0].path, "A");
    assert_eq!(result.includes[1].path, "std/A");

    assert_eq!(result.links.len(), 1);
    assert_eq!(result.links[0].ident.raw(), "NewLink");
    assert_eq!(
        (
            result.links[0].bitrate,
            result.links[0].latency,
            result.links[0].jitter
        ),
        (300, 0.1, 0.1)
    );

    assert_eq!(result.modules.len(), 2);

    assert_eq!(result.modules[0].ident.raw(), "SubM");
    assert_eq!(result.modules[0].gates.len(), 1);
    assert_eq!(result.modules[0].gates[0].name, "another");
    assert_eq!(result.modules[0].gates[0].size, 5);

    assert_eq!(result.modules[0].parameters.len(), 1);
    assert_eq!(result.modules[0].parameters[0].ident, "addr");
    assert_eq!(result.modules[0].parameters[0].ty, "usize");

    assert_eq!(result.modules[1].ident.raw(), "Main");
    assert_eq!(result.modules[1].gates.len(), 3);
    assert_eq!(result.modules[1].gates[0].name, "some");
    assert_eq!(result.modules[1].gates[0].size, 5);
    assert_eq!(result.modules[1].gates[1].name, "same");
    assert_eq!(result.modules[1].gates[1].size, 5);
    assert_eq!(result.modules[1].gates[2].name, "sike");
    assert_eq!(result.modules[1].gates[2].size, 1);

    assert_eq!(result.modules[1].submodules.len(), 1);
    assert_eq!(result.modules[1].submodules[0].ty.inner(), "SubM");
    assert_eq!(result.modules[1].submodules[0].desc.descriptor, "m");

    assert_eq!(result.modules[1].connections.len(), 2);
    assert_eq!(result.modules[1].connections[0].channel, None);

    assert_eq!(result.subsystems.len(), 1);
    assert_eq!(result.subsystems[0].ident.raw(), "SimMain");

    assert_eq!(result.subsystems[0].nodes.len(), 1);
    assert_eq!(result.subsystems[0].nodes[0].desc.descriptor, "router");
    assert_eq!(result.subsystems[0].nodes[0].ty.inner(), "Main");
}

// #[test]
// fn ndl_desugar_test() {
//     use crate::*;

//     let mut resolver = NdlResolver::new("tests/TycTest").expect("Failed to load TcyTest");
//     let _ = resolver.run();

//     let unit = resolver.units.get("Main").unwrap();

//     let desugared_unit = desugar_unit(unit, &resolver);

//     println!("{}", unit);
//     println!("{}", desugared_unit);
// }

// #[test]
// fn ndl_tycheck_test() {
//     use crate::*;

//     let mut resolver =
//         NdlResolver::new("./tests/TycTest").expect("Failed to create resovler with valid root.");

//     let _ = resolver.run();

//     println!("{}", resolver);

//     let unit = resolver.expanded_units.get("Main").unwrap();

//     let _res = validate(unit, &resolver);
// }

#[test]
fn ndl_full_test() {
    let mut resolver = NdlResolver::quiet("tests/full")
        .expect("Failed to create resolver")
        .verbose("tests/full/output/");

    println!("{}", resolver);

    let _ = resolver.run();

    println!("{}", resolver);
}

#[test]
fn ndl_protsim_test() {
    let mut resolver = NdlResolver::quiet("tests/protsim").expect("Failed to create resolver");

    println!("{}", resolver);

    let _ = resolver.run();

    println!("{}", resolver);
}

///
/// Tests all synatx & semantic errors concerning links.
///
mod link;

///
/// Tests for all syntax & sematntics errors concerning prototyping.
///
mod prototype;

///
/// Tests for all syntax & sematntics errors concerning modules.
///
mod module;

///
/// Tests for all syntax & semantic errors concerning networks.
///
mod subsystem;

mod include;
mod lookalike;

#[macro_export]
macro_rules! check_err {
    ($e:expr => $code:ident, $msg:literal, $transient:literal, $solution:expr) => {
        assert_eq!($e.code, $code);
        assert_eq!($e.msg, $msg);
        assert_eq!($e.transient, $transient);
        assert_eq!($e.solution, $solution);
    };
}
