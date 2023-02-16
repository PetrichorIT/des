use des_ndl::error::*;
use des_ndl::ir::Cluster;
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn module_ast_baseline() -> RootResult<()> {
    let mut ctx = Context::load("tests/module_ast_baseline.ndl")?;
    let entry = ctx.entry.take().unwrap();

    assert_eq!(entry.ident.raw, "Main");

    assert_eq!(entry.gates[0].ident.raw, "uplink");
    assert_eq!(entry.gates[0].cluster, Cluster::Clusted(2));
    assert_eq!(entry.gates[1].ident.raw, "downlink");
    assert_eq!(entry.gates[1].cluster, Cluster::Clusted(2));

    assert_eq!(
        entry.submodules[0].typ.as_module().unwrap().ident.raw,
        "Sub"
    );
    let sub = entry.submodules[0].typ.as_module().unwrap();
    assert_eq!(sub.connections, vec![]);
    assert_eq!(sub.submodules, vec![]);

    assert_eq!(sub.gates[0].ident.raw, "in");
    assert_eq!(sub.gates[0].cluster, Cluster::Standalone);
    assert_eq!(sub.gates[1].ident.raw, "out");
    assert_eq!(sub.gates[1].cluster, Cluster::Standalone);

    Ok(())
}

#[test]
fn module_ast_noident() {
    let err = Context::load("tests/module_ast_noident.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "unexpected token for module symbol: expected <ident>, found delim"
    );
}

#[test]
fn module_ast_noident2() {
    let err = Context::load("tests/module_ast_noident2.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "unexpected token for module symbol: expected <ident>, found <literal>"
    );
}

#[test]
fn module_ast_nodelim() {
    let err = Context::load("tests/module_ast_nodelim.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ExpectedDelimited,
        "expected delimited sequence, found EOF"
    );
}

#[test]
fn module_ast_nodelim2() {
    let err = Context::load("tests/module_ast_nodelim2.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ExpectedDelimited,
        "expected delimited sequence, found ';'"
    );
}
