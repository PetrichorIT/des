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
    println!("{err}");

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
    println!("{err}");

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
    println!("{err}");

    let errs = err.errors;
    assert_eq!(errs.len(), 1);

    check_err!(errs.get(0) =>
        ErrorKind::ModuleLocalCyclicDeps,
        "found cyclic definition of local modules: A <- B <- C <- A"
    );
}
