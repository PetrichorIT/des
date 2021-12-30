use des_ndl::*;

fn main() {
    // This application should be perfed by flamegraph
    for _ in 0..100 {
        let mut resolver = NdlResolver::new("examples").expect("Failed resolver");
        resolver.run().expect("Failed to run");

        drop(resolver)
    }
}
