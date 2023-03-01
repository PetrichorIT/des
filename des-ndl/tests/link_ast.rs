use des_ndl::error::*;
use des_ndl::ir::{Item, Literal};
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn link_ast_baseline() -> RootResult<()> {
    let ctx = Context::load("tests/link_ir_baseline.ndl")?;
    let ir = ctx.ir.values().collect::<Vec<_>>()[0];

    assert!(matches!(ir.items[0], Item::Link(_)));
    assert!(matches!(ir.items[1], Item::Link(_)));
    assert!(matches!(ir.items[2], Item::Link(_)));

    let a = ir.link("A").unwrap();
    let b = ir.link("B").unwrap();
    let c = ir.link("C").unwrap();

    assert_eq!(a.ident.raw, "A");
    assert_eq!(b.ident.raw, "B");
    assert_eq!(c.ident.raw, "C");

    assert_eq!(a.jitter, 0.2);
    assert_eq!(b.jitter, 0.2);
    assert_eq!(c.jitter, 1.0);

    assert_eq!(a.fields.get("bparam"), None);
    assert_eq!(
        b.fields.get("bparam"),
        Some(&Literal::String("string".to_string()))
    );
    assert_eq!(
        c.fields.get("bparam"),
        Some(&Literal::String("strong".to_string()))
    );

    Ok(())
}

#[test]
fn link_ast_noident() {
    let err = Context::load("tests/link_ast_noident.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "unexpected token for link symbol: expected <ident>, found delim"
    );
}

#[test]
fn link_ast_invalid_inh() {
    let err = Context::load("tests/link_ast_invalid_inh.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "failed to parse value in joined statement: expected <ident>, found delim"
    );
}

#[test]
fn link_ast_invalid_kv() {
    let err = Context::load("tests/link_ast_invalid_kv.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "missing delimiter in key-value pair: expected ':', found <literal>"
    );
}

#[test]
fn link_ast_invalid_kv2() {
    let err = Context::load("tests/link_ast_invalid_kv2.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "missing value in key-value pair: expected <literal>, found <ident>"
    );
}

#[test]
fn link_ast_nodelim() {
    let err = Context::load("tests/link_ast_nodelim.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ExpectedDelimited,
        "expected delimited sequence, found 'module'"
    );
}
