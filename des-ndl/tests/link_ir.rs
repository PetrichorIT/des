use des_ndl::error::*;
use des_ndl::ir::{Item, Literal};
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn link_ir_baseline() -> RootResult<()> {
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
fn link_ir_inh() {
    let err = Context::load("tests/link_ir_inh.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 2);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolNotFound,
        "did not find link symbol 'C', not in scope"
    );

    check_err!(errs.get(1) =>
        ErrorKind::SymbolNotFound,
        "did not find link symbol 'C', not in scope"
    );
}

#[test]
fn link_ir_inh_with_solutions() {
    let err = Context::load("tests/link_ir_inh2/main.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 2);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolNotFound,
        "did not find link symbol 'C', not in scope",
        "try including 'C' from '../sub1'"

    );

    check_err!(errs.get(1) =>
        ErrorKind::SymbolNotFound,
        "did not find link symbol 'C', not in scope",
        "try including 'C' from '../sub1'"
    );
}

#[test]
fn link_ir_inh_dup() {
    let err = Context::load("tests/link_ir_inh_dup.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::LinkInheritanceDuplicatedSymbols,
        "found duplicated symbol 'C' in link inheritance statement"
    );
}

#[test]
fn link_ir_known_values() {
    let err = Context::load("tests/link_ir_known_values.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::LinkKnownKeysInvalidValue,
        "known key 'jitter' expects a value of type float"
    );
}

#[test]
fn link_ir_local_dup() {
    let err = Context::load("tests/link_ir_local_dup.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolDuplication,
        "cannot create new symbol 'A', was allready defined"
    );
}

#[test]
fn link_ir_nonlocal_dup() {
    let err = Context::load("tests/link_ir_nonlocal_dup/main.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolDuplication,
        "found duplicated symbol 'A', with 1 duplications"
    );
}

#[test]
fn link_ir_requried_values() {
    let err = Context::load("tests/link_ir_required_values.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 2);

    check_err!(errs.get(0) =>
        ErrorKind::LinkMissingRequiredFields,
        "missing required field 'latency', was not defined locally or in prototypes"
    );

    check_err!(errs.get(1) =>
        ErrorKind::LinkMissingRequiredFields,
        "missing required field 'bitrate', was not defined locally or in prototypes"
    );
}
