use des_ndl::*;
use utils::bench::*;

fn main() {
    let mut ctx = BenchmarkCtx::new("tyc_stage (including lex + par)", 1_000);

    bench(&mut ctx, "small error free", || {
        let _ = NdlResolver::new_with("benches/tyc_stage/example_1", NdlResolverOptions::bench())
            .expect("Failed to load resolver")
            .run();
    });

    bench(&mut ctx, "big error free", || {
        let _ = NdlResolver::new_with("benches/tyc_stage/example_2", NdlResolverOptions::bench())
            .expect("Failed to load resolver")
            .run();
    });

    bench(&mut ctx, "small error prone", || {
        let _ = NdlResolver::new_with("benches/tyc_stage/example_3", NdlResolverOptions::bench())
            .expect("Failed to load resolver")
            .run();
    });

    bench(&mut ctx, "big error prone", || {
        let _ = NdlResolver::new_with("benches/tyc_stage/example_4", NdlResolverOptions::bench())
            .expect("Failed to load resolver")
            .run();
    });

    ctx.finish(true)
        .expect("Failed to write benchmarks to file")
}
