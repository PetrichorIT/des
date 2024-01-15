use des_ndl::error::*;
use des_ndl::ir::Cluster;
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn gates_ir_baseline() -> RootResult<()> {
    let mut ctx = Context::load("tests/gates_ir_baseline.ndl")?;
    let entry = ctx.entry.take().unwrap();

    assert_eq!(entry.ident.raw, "M");
    assert_eq!(entry.gates.len(), 4);

    assert_eq!(entry.gates[0].ident.raw, "in");
    assert_eq!(entry.gates[0].cluster, Cluster::Standalone);

    assert_eq!(entry.gates[1].ident.raw, "out");
    assert_eq!(entry.gates[1].cluster, Cluster::Standalone);

    assert_eq!(entry.gates[2].ident.raw, "influx");
    assert_eq!(entry.gates[2].cluster, Cluster::Clusted(5));

    assert_eq!(entry.gates[3].ident.raw, "outflow");
    assert_eq!(entry.gates[3].cluster, Cluster::Clusted(1));

    Ok(())
}

#[test]
fn gates_ir_local_dup() {
    let err = Context::load("tests/gates_ir_local_dup.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolDuplication,
        "gate(-cluster) 'in' was defined multiple times"
    );
}
