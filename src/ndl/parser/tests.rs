#[test]
fn t() {
    use crate::ndl::parser::Parser;

    let mut parser = Parser::new(String::from("./src/ndl/examples/NetworkStack.ndl"));

    let success = parser.parse();

    if success {
        println!("{:?}", parser);
    } else {
        parser.print_errors().expect("Failedio")
    }
}
