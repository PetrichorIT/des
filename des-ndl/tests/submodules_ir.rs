use des_ndl::error::*;
use des_ndl::ir::Cluster;
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn submodules_ir_baseline() -> RootResult<()> {
    let mut ctx = Context::load("tests/submodules_ir_baseline.ndl")?;
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
fn submodules_ir_local_dup() {
    let err = Context::load("tests/submodules_ir_local_dup.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolDuplication,
        "submodule(-cluster) 'a' was defined multiple times"
    );
}

#[test]
fn submodules_ir_unknown_ty() {
    let err = Context::load("tests/submodules_ir_unknown_ty.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolNotFound,
        "did not find submodule symbol 'A', not in scope"
    );
}

#[test]
fn submodules_ir_unknown_ty_soloution() {
    let err = Context::load("tests/submodules_ir_unknown_ty_solution/main.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolNotFound,
        "did not find submodule symbol 'A', not in scope",
        "try including 'A' from '../sub1'"
    );
}
