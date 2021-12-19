#[test]
fn it_works() {
    use crate::ndl::*;

    let mut resolver =
        NdlResolver::new("src/ndl/examples").expect("Failed to create resovler with valid root.");

    resolver.run();

    println!("{}", resolver);

    let unit = resolver.units.get("NetworkNode").unwrap();

    let tyctx = TyContext::new();

    let _res = tycheck::validate(&resolver, unit, &tyctx);
}
