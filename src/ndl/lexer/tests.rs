#[test]
fn test_parser() {
    use super::tokenize;

    let contents = std::fs::read_to_string("./src/ndl/examples/Test.ndl").expect("msg");

    let res = tokenize(&contents);
    for r in res {
        println!("{:?}", r)
    }
}
