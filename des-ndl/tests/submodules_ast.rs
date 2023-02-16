use des_ndl::error::*;
use des_ndl::ir::Cluster;
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn submodules_ast_baseline() -> RootResult<()> {
    let mut ctx = Context::load("tests/submodules_ast_baseline.ndl")?;
    let entry = ctx.entry.take().unwrap();

    assert_eq!(entry.ident.raw, "M");
    assert_eq!(entry.submodules.len(), 3);

    assert_eq!(entry.submodules[0].ident.raw, "a");
    assert_eq!(entry.submodules[0].typ.as_module().unwrap().ident.raw, "A");
    assert_eq!(entry.submodules[0].cluster, Cluster::Standalone);

    assert_eq!(entry.submodules[1].ident.raw, "b");
    assert_eq!(entry.submodules[1].typ.as_module().unwrap().ident.raw, "A");
    assert_eq!(entry.submodules[1].cluster, Cluster::Standalone);

    assert_eq!(entry.submodules[2].ident.raw, "c");
    assert_eq!(entry.submodules[2].typ.as_module().unwrap().ident.raw, "A");
    assert_eq!(entry.submodules[2].cluster, Cluster::Clusted(4));

    Ok(())
}

#[test]
fn submodules_ast_nodelim() {
    let err = Context::load("tests/submodules_ast_nodelim.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ExpectedDelimited,
        "expected delimited sequence, found ';'"
    );
}

#[test]
fn submodules_ast_wrong_delim() {
    let err = Context::load("tests/submodules_ast_wrong_delim.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedDelim,
        "expected delimited sequence '{ ... }', found delimited sequence '( ... )'"
    );
}

#[test]
fn submodules_ast_symbol_dup() {
    let err = Context::load("tests/submodules_ast_symbol_dup.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleSubDuplicatedSymbols,
        "submodule(-cluster) 'a' was defined multiple times"
    );
}

#[test]
fn submodules_ast_missing_ty() {
    let err = Context::load("tests/submodules_ast_missing_ty.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "expected <ident>, found ','"
    );
}
