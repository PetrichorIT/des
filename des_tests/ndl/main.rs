use des_ndl::NdlResolver;

mod alice;

fn main() {
    let mut r = NdlResolver::new("src/ndl/examples").expect("Failed to create workspace");
    let _ = r.run();
    // println!("{}", r);
}
