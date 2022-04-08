use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ndl::*;

fn lexer_tokenize(c: &mut Criterion) {
    const KB: usize = 1024;

    let mut group = c.benchmark_group("ndl::lexer::by_input_size");
    for size in [1, 2, 4, 8, 16] {
        let str = std::fs::read_to_string(format!("benches/lexer/{}kb.ndl", size)).unwrap();

        group.throughput(Throughput::Bytes(size * KB as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size)),
            &size,
            |b, _size| {
                b.iter(|| {
                    let token_stream = tokenize(&str, 0);
                    let _ = token_stream.collect::<Vec<_>>();
                })
            },
        );
    }
    group.finish()
}

criterion_group!(benches, lexer_tokenize);
criterion_main!(benches);
