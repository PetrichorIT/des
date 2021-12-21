use des_ndl::*;
use utils::bench::*;

fn main() {
    let mut ctx = BenchmarkCtx::new("lex_stage", 10000);

    let mut ectx = GlobalErrorContext::new();
    let asset = SourceAsset::load(SourceAssetDescriptor::new(
        "./benches/lex_stage/example_1.ndl".into(),
        "main".into(),
    ))
    .expect("failed to load asset");

    bench(&mut ctx, "tokensize_and_parse (small valid)", || {
        let _ = tokenize_and_validate(black_box(&asset), &mut ectx);
    });

    let mut ectx = GlobalErrorContext::new();
    let asset = SourceAsset::load(SourceAssetDescriptor::new(
        "./benches/lex_stage/example_2.ndl".into(),
        "main".into(),
    ))
    .expect("failed to load asset");

    bench(&mut ctx, "tokensize_and_parse (big valid)", || {
        let _ = tokenize_and_validate(black_box(&asset), &mut ectx);
    });

    let mut ectx = GlobalErrorContext::new();
    let asset = SourceAsset::load(SourceAssetDescriptor::new(
        "./benches/lex_stage/example_3.ndl".into(),
        "main".into(),
    ))
    .expect("failed to load asset");

    bench(&mut ctx, "tokensize_and_parse (small invalid)", || {
        let _ = tokenize_and_validate(black_box(&asset), &mut ectx);
    });

    let mut ectx = GlobalErrorContext::new();
    let asset = SourceAsset::load(SourceAssetDescriptor::new(
        "./benches/lex_stage/example_4.ndl".into(),
        "main".into(),
    ))
    .expect("failed to load asset");

    bench(&mut ctx, "tokensize_and_parse (big invalid)", || {
        let _ = tokenize_and_validate(black_box(&asset), &mut ectx);
    });

    ctx.finish(true)
        .expect("Failed to record benchmark results");
}
