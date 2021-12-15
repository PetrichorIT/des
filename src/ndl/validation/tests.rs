#[test]
fn it_works() {
    use crate::ndl::*;

    let mut resolver =
        NdlResolver::new("src/ndl/examples").expect("Failed to create resovler with valid root.");

    resolver.parse();

    println!("{}", resolver);

    let unit = resolver.units.get("NetworkNode").unwrap();

    let _res = validation::validate(&resolver, unit);
}
