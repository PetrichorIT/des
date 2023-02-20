use des_ndl::ast::ModuleTypus;
use des_ndl::error::*;
use des_ndl::ir::Item;
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn dyn_baseline() -> RootResult<()> {
    let mut ctx = Context::load("tests/dyn_baseline.ndl")?;
    let entry = ctx.entry.take().unwrap();
    let ir = &ctx.ir.values().collect::<Vec<_>>()[0].items;
    let Item::Module(ref basic) = ir[0] else {
        unreachable!()
    };
    assert_eq!(basic.ident.raw, "Basic");
    assert_eq!(
        basic
            .inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        Vec::<String>::new()
    );
    assert_eq!(basic.ast.typus(), ModuleTypus::Primal);

    let Item::Module(ref a) = ir[1] else {
        unreachable!()
    };
    assert_eq!(a.ident.raw, "A");
    assert_eq!(
        a.inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        vec!["Basic".to_string()]
    );
    assert_eq!(a.ast.typus(), ModuleTypus::Inherited);

    let Item::Module(ref b) = ir[2] else {
        unreachable!()
    };
    assert_eq!(b.ident.raw, "B");
    assert_eq!(
        b.inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        vec!["Basic".to_string()]
    );
    assert_eq!(b.ast.typus(), ModuleTypus::Inherited);

    let Item::Module(ref dyn_m) = ir[3] else {
        unreachable!()
    };
    assert_eq!(dyn_m.ident.raw, "Dyn");
    assert_eq!(
        dyn_m
            .inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        Vec::<String>::new()
    );
    assert_eq!(dyn_m.ast.typus(), ModuleTypus::Dynamic);

    // follow entry
    assert_eq!(entry.ident.raw, "M");
    assert_eq!(entry.gates.len(), 0);
    assert_eq!(entry.submodules[0].dynamic, false);

    let sub1 = entry.submodules[0].typ.as_module_arc().unwrap();
    assert_eq!(sub1.ident.raw, "Dyn");
    assert_eq!(sub1.gates.len(), 0);
    assert_eq!(sub1.submodules[0].dynamic, false);

    let sub2 = sub1.submodules[0].typ.as_module_arc().unwrap();
    assert_eq!(sub2.ident.raw, "A");
    assert_eq!(sub2.gates.len(), 2);
    // assert_eq!(sub2.submodules[0].dynamic, false);

    Ok(())
}

#[test]
fn dyn_constraint_broken() {
    let err = Context::load("tests/dyn_constraint_broken.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleDynConstraintsBroken,
        "module 'A' does not inherit 'Basic', thus cannot be assigned to dyn field 'sub'"
    );
}

#[test]
fn dyn_non_resovle_dyn_spec() {
    let err = Context::load("tests/dyn_non_resovle_dyn_spec.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolNotFound,
        "did not find submodule symbol 'WrongSymbol', not in scope"
    );

    // This test ensures that the dyn clause is completely ignored, thus throwing no errors
    // additionally the symbol M should still exist

    // thus T whould be fully valid
}

#[test]
fn dyn_unknown_key() {
    let err = Context::load("tests/dyn_unknown_key.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 2);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolNotFound,
        "did not find submodule symbol for dyn-spec 'wrongkey', not in scope"
    );

    check_err!(errs.get(1) =>
        ErrorKind::ModuleDynNotResolved,
        "missing specification for dynamic members of submodule 'd': missing fields 'sub'"
    );
}

#[test]
fn dyn_unknown_value() {
    let err = Context::load("tests/dyn_unknown_value.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::SymbolNotFound,
        "did not find dyn-spec submodule symbol 'Wrong', not in scope"
    );
}

#[test]
fn dyn_extending_override() -> RootResult<()> {
    let mut ctx = Context::load("tests/dyn_extending_override.ndl")?;
    let entry = ctx.entry.take().unwrap();
    let ir = &ctx.ir.values().collect::<Vec<_>>()[0].items;
    let Item::Module(ref basic) = ir[0] else {
        unreachable!()
    };
    assert_eq!(basic.ident.raw, "Basic");
    assert_eq!(
        basic
            .inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        Vec::<String>::new()
    );
    assert_eq!(basic.ast.typus(), ModuleTypus::Primal);

    let Item::Module(ref a) = ir[1] else {
        unreachable!()
    };
    assert_eq!(a.ident.raw, "A");
    assert_eq!(
        a.inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        vec!["Basic".to_string()]
    );
    assert_eq!(a.ast.typus(), ModuleTypus::Inherited);

    let Item::Module(ref b) = ir[2] else {
        unreachable!()
    };
    assert_eq!(b.ident.raw, "B");
    assert_eq!(
        b.inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        vec!["Basic".to_string()]
    );
    assert_eq!(b.ast.typus(), ModuleTypus::Inherited);

    let Item::Module(ref dyn_m) = ir[3] else {
        unreachable!()
    };
    assert_eq!(dyn_m.ident.raw, "Dyn");
    assert_eq!(
        dyn_m
            .inherited
            .iter()
            .map(|s| s.raw().raw.clone())
            .collect::<Vec<String>>(),
        Vec::<String>::new()
    );
    assert_eq!(dyn_m.ast.typus(), ModuleTypus::Dynamic);

    // follow entry
    assert_eq!(entry.ident.raw, "M");
    assert_eq!(entry.gates.len(), 0);
    assert_eq!(entry.submodules[0].dynamic, false);

    let sub1 = entry.submodules[0].typ.as_module_arc().unwrap();
    assert_eq!(sub1.ident.raw, "Dyn");
    assert_eq!(sub1.gates.len(), 0);
    assert_eq!(sub1.submodules[0].dynamic, false);

    let sub2 = sub1.submodules[0].typ.as_module_arc().unwrap();
    assert_eq!(sub2.ident.raw, "A");
    assert_eq!(sub2.gates.len(), 3);
    // assert_eq!(sub2.submodules[0].dynamic, false);

    Ok(())
}

#[test]
fn dyn_not_resolved() {
    let err = Context::load("tests/dyn_not_resolved.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 2);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleDynNotResolved,
        "missing specification for dynamic members of submodule 'd': missing fields 'sub'"
    );

    check_err!(errs.get(1) =>
        ErrorKind::ModuleDynNotResolved,
        "missing specification for dynamic members of submodule 'b': missing fields 'a, c'"
    );
}
