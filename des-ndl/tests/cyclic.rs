use des_ndl::error::*;
use des_ndl::*;

#[macro_use]
mod common;

#[test]
fn cyclic_baseline() -> RootResult<()> {
    let mut ctx = Context::load("tests/cyclic_baseline/main.ndl")?;
    let entry = ctx.entry.take().unwrap();

    assert_eq!(entry.ident.raw, "M");
    assert_eq!(entry.submodules[0].typ.as_module().unwrap().ident.raw, "A");
    assert_eq!(entry.submodules[1].typ.as_module().unwrap().ident.raw, "B");

    Ok(())
}

#[test]
fn cyclic_includes() {
    let err = Context::load("tests/cyclic_includes/main.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::CyclicDeps,
        "found cyclic includes: sub1 <- sub2 <- sub3 <- sub1"
    );
}

#[test]
fn cyclic_local_links() {
    let err = Context::load("tests/cyclic_local_links.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::LinkLocalCyclicDeps,
        "found cyclic definition of local links: A <- B <- C <- A"
    );
}

#[test]
fn cyclic_local_modules() {
    let err = Context::load("tests/cyclic_local_modules.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleLocalCyclicDeps,
        "found cyclic definition of local modules: A <- B <- C <- A"
    );
}

#[test]
fn cyclic_local_selfreferential() {
    let err = Context::load("tests/cyclic_local_selfreferential.ndl").unwrap_err();
    // println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 4);

    check_err!(errs.get(0) =>
        ErrorKind::LinkLocalCyclicDeps,
        "found cyclic definition of local links: LDirect <- LDirect"
    );

    check_err!(errs.get(1) =>
        ErrorKind::LinkLocalCyclicDeps,
        "found cyclic definition of local links: LIndirect1 <- LIndirect2 <- LIndirect1"
    );

    check_err!(errs.get(2) =>
        ErrorKind::ModuleLocalCyclicDeps,
        "found cyclic definition of local modules: Direct <- Direct"
    );

    check_err!(errs.get(3) =>
        ErrorKind::ModuleLocalCyclicDeps,
        "found cyclic definition of local modules: IndirectA <- IndirectB <- IndirectA"
    );
}

#[test]
fn cyclic_module_inh() {
    let err = Context::load("tests/cyclic_module_inh.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 2);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleLocalCyclicDeps,
        "found cyclic definition of local modules: A <- C <- B <- A"
    );

    check_err!(errs.get(1) =>
        ErrorKind::ModuleLocalCyclicDeps,
        "found cyclic definition of local modules: H1 <- H2 <- H1"
    );
}

#[test]
fn cyclic_dependable() {
    let err = Context::load("tests/cyclic_dependable/main.ndl").unwrap_err();
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 2);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleLocalCyclicDeps,
        "found cyclic definition of local modules: A <- B <- A"
    );

    // cause cyclic will not be loaded
    check_err!(errs.get(1) =>
        ErrorKind::SymbolNotFound,
        "did not find inheritance symbol 'B', not in scope",
        "try including 'B' from '../sub'"
    );
}
