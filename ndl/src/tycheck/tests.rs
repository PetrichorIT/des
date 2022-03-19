#[test]
fn it_works() {
    use crate::*;

    let mut resolver =
        NdlResolver::new("./tests/TycTest").expect("Failed to create resovler with valid root.");

    let _ = resolver.run();

    println!("{}", resolver);

    let unit = resolver.desugared_units.get("Main").unwrap();

    let _res = tycheck::validate(unit, &resolver);
}
