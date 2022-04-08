use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ndl::*;

fn parser_parse(c: &mut Criterion) {
    const KB: usize = 1024;

    let mut smap = SourceMap::new();

    let mut group = c.benchmark_group("ndl::parser::by_input_size");
    for size in [1, 2, 4, 8] {
        // let str = std::fs::read_to_string(format!("benches/parser/{}kb.ndl", size)).unwrap();

        let asset = smap
            .load(AssetDescriptor::from_path(
                format!("benches/parser/{}kb.ndl", size).into(),
                &PathBuf::from("benches/parser/"),
            ))
            .unwrap();
        let mut ectx = GlobalErrorContext::new();

        let token_stream = tokenize_and_validate(asset, &mut ectx).unwrap();

        group.throughput(Throughput::Bytes(size * KB as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size)),
            &size,
            |b, _size| {
                b.iter(|| {
                    let _ = parse(asset, token_stream.clone());
                })
            },
        );
    }
    group.finish()
}

criterion_group!(benches, parser_parse);
criterion_main!(benches);
