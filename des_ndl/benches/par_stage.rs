use des_ndl::*;
use utils::bench::*;

pub fn main() {
    let mut ctx = BenchmarkCtx::new("par_bench", 10_000);

    {
        let mut ectx = GlobalErrorContext::new();
        let asset = SourceAsset::load(SourceAssetDescriptor::new(
            "./benches/par_stage/example_1.ndl".into(),
            "main".into(),
        ))
        .expect("failed to load asset");

        let token_stream = tokenize_and_validate(&asset, &mut ectx).expect("failed to lex");

        bench(&mut ctx, "parse (small valid)", || {
            let _ = parse(&asset, token_stream.clone());
        });
    }

    {
        let mut ectx = GlobalErrorContext::new();
        let asset = SourceAsset::load(SourceAssetDescriptor::new(
            "./benches/par_stage/example_2.ndl".into(),
            "main".into(),
        ))
        .expect("failed to load asset");

        let token_stream = tokenize_and_validate(&asset, &mut ectx).expect("failed to lex");

        bench(&mut ctx, "parse (big valid)", || {
            let _ = parse(&asset, token_stream.clone());
        });
    }

    {
        let mut ectx = GlobalErrorContext::new();
        let asset = SourceAsset::load(SourceAssetDescriptor::new(
            "./benches/par_stage/example_3.ndl".into(),
            "main".into(),
        ))
        .expect("failed to load asset");

        let token_stream = tokenize_and_validate(&asset, &mut ectx).expect("failed to lex");

        bench(&mut ctx, "parse (small invalid)", || {
            let _ = parse(&asset, token_stream.clone());
        });
    }

    {
        let mut ectx = GlobalErrorContext::new();
        let asset = SourceAsset::load(SourceAssetDescriptor::new(
            "./benches/par_stage/example_4.ndl".into(),
            "main".into(),
        ))
        .expect("failed to load asset");

        let token_stream = tokenize_and_validate(&asset, &mut ectx).expect("failed to lex");

        bench(&mut ctx, "parse (big invalid)", || {
            let _ = parse(&asset, token_stream.clone());
        });
    }

    ctx.finish(true)
        .expect("Failed to write benchmarks to file")
}
