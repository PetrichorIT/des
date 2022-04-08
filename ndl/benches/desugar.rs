use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ndl::*;

fn desugar_bench(c: &mut Criterion) {
    const KB: usize = 1024;

    let mut group = c.benchmark_group("ndl::desugar::by_input_size");
    for size in [1, 2, 4, 8] {
        // let str = std::fs::read_to_string(format!("benches/parser/{}kb.ndl", size)).unwrap();

        let mut resolver = NdlResolver::new_with(
            &format!("benches/parser/{}kb.ndl", size),
            NdlResolverOptions {
                silent: false,
                verbose: false,
                verbose_output_dir: PathBuf::new(),
                desugar: false,
                tychk: false,
            },
        )
        .unwrap();
        let _ = resolver.run();

        group.throughput(Throughput::Bytes(size * KB as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size)),
            &size,
            |b, _size| {
                b.iter(|| {
                    desugar(&mut resolver);
                    resolver.desugared_units.clear();
                    resolver.ectx.desugaring_errors.clear();
                })
            },
        );
    }
    group.finish()
}

criterion_group!(benches, desugar_bench);
criterion_main!(benches);
