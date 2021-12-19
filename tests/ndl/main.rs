use dse::ndl::NdlResolver;

fn main() {
    let mut r = NdlResolver::new("src/ndl/examples").expect("Failed to create workspace");
    r.run();
    // println!("{}", r);
}
