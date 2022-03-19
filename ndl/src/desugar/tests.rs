#[test]
fn desugar_test() {
    use crate::*;

    let mut resolver = NdlResolver::new("tests/TycTest").expect("Failed to load TcyTest");
    let _ = resolver.run();

    let unit = resolver.units.get("Main").unwrap();
    let desugared_unit = desugar(unit, &resolver);

    println!("{}", unit);
    println!("{}", desugared_unit);
}
