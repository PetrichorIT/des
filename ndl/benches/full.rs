use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ndl::*;

fn full(c: &mut Criterion) {
    const KB: usize = 1024;

    let mut group = c.benchmark_group("ndl::full::by_input_size");
    for size in [1, 2, 4, 8] {
        // let str = std::fs::read_to_string(format!("benches/parser/{}kb.ndl", size)).unwrap();

        let mut resolver = NdlResolver::new_with(
            &format!("benches/parser/{}kb.ndl", size),
            NdlResolverOptions {
                silent: false,
                verbose: false,
                verbose_output_dir: PathBuf::new(),
                desugar: true,
                tychk: false,
            },
        )
        .unwrap();
        resolver.preload();

        group.throughput(Throughput::Bytes(size * KB as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size)),
            &size,
            |b, _size| {
                b.iter(|| {
                    let mut r = resolver.clone();
                    r.run()
                })
            },
        );
    }
    group.finish()
}

criterion_group!(benches, full);
criterion_main!(benches);
