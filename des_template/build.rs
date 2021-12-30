use des_ndl::*;

fn main() {
    let mut resolver = NdlResolver::new("./src").expect("Should not find src directory.");
    resolver.run().expect("Failed to run resolver.");

    if resolver.ectx.has_errors() {
        std::process::abort()
    } else {
        // for scope in &resolver.scopes {
        //     println!("cargo:rerun-if-changed={}", scope.to_str().unwrap())
        // }
    }
}
