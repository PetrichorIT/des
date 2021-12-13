use dse::ndl::parser::Parser;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() < 2 {
        eprintln!("Missing input parameter <filename...>");
        return;
    }
    let filename = argv[1].clone();

    let mut parser = Parser::new(filename);
    let success = parser.parse();

    if success {
        println!("{}", parser);
    } else {
        parser.print_errors().unwrap();
    }
}
