use des_ndl::error::*;
use des_ndl::ir::{Cluster, GateServiceType};
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn gates_ast_baseline() -> RootResult<()> {
    let mut ctx = Context::load("tests/gates_ast_baseline.ndl")?;
    let entry = ctx.entry.take().unwrap();

    assert_eq!(entry.ident.raw, "M");
    assert_eq!(entry.gates.len(), 4);

    assert_eq!(entry.gates[0].ident.raw, "in");
    assert_eq!(entry.gates[0].cluster, Cluster::Standalone);
    assert_eq!(entry.gates[0].service_typ, GateServiceType::Input);

    assert_eq!(entry.gates[1].ident.raw, "out");
    assert_eq!(entry.gates[1].cluster, Cluster::Standalone);
    assert_eq!(entry.gates[1].service_typ, GateServiceType::None);

    assert_eq!(entry.gates[2].ident.raw, "influx");
    assert_eq!(entry.gates[2].cluster, Cluster::Clusted(5));
    assert_eq!(entry.gates[2].service_typ, GateServiceType::None);

    assert_eq!(entry.gates[3].ident.raw, "outflow");
    assert_eq!(entry.gates[3].cluster, Cluster::Clusted(1));
    assert_eq!(entry.gates[3].service_typ, GateServiceType::Output);

    Ok(())
}

#[test]
fn gates_ast_nodelim() {
    let err = Context::load("tests/gates_ast_nodelim.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ExpectedDelimited,
        "expected delimited sequence, found ';'"
    );
}

#[test]
fn gates_ast_wrong_delim() {
    let err = Context::load("tests/gates_ast_wrong_delim.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedDelim,
        "expected delimited sequence '{ ... }', found delimited sequence '[ ... ]'"
    );
}

#[test]
fn gates_ast_invalid_annotation() {
    let err = Context::load("tests/gates_ast_invalid_annotation.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::InvalidAnnotation,
        "invalid annotation 'outflux', gates can only be annoted with input/output"
    );
}

#[test]
fn gates_ast_symbol_dup() {
    let err = Context::load("tests/gates_ast_symbol_dup.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleGatesDuplicatedSymbols,
        "gate(-cluster) 'influx' was defined multiple times"
    );
}

#[test]
fn gates_ast_invalid_cluster() {
    let err = Context::load("tests/gates_ast_invalid_cluster.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 4);

    check_err!(errs.get(0) =>
        ErrorKind::InvalidLitTyp,
        "cannot create gate-cluster with literal of type float, expected literal of type integer"
    );

    check_err!(errs.get(1) =>
        ErrorKind::InvalidLitTyp,
        "cannot create gate-cluster with literal of type string, expected literal of type integer"
    );

    check_err!(errs.get(2) =>
        ErrorKind::ModuleGatesInvalidClusterSize,
        "cannot create gate-cluster of size '0', requires positiv integer"
    );

    check_err!(errs.get(3) =>
        ErrorKind::ModuleGatesInvalidClusterSize,
        "cannot create gate-cluster of size '-1', requires positiv integer"
    );
}

#[test]
fn gates_ast_invalid_punct() {
    let err = Context::load("tests/gates_ast_invalid_punct.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::UnexpectedToken,
        "expected <ident>, found ','"
    );
}
