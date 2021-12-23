#[test]
fn it_works() {
    use crate::*;

    let mut resolver =
        NdlResolver::new("./tests/TycTest").expect("Failed to create resovler with valid root.");

    let _ = resolver.run();

    println!("{}", resolver);

    let unit = resolver.units.get("Main").unwrap();

    let tyctx = TyContext::new();

    let _res = tycheck::validate(&resolver, unit, &tyctx);
}
