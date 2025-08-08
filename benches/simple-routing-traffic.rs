use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use des::{
    net::Sim,
    prelude::{send, Message, Module, Registry},
    runtime::{random, Builder},
};

struct Host {
    n: usize,
}

impl Host {
    fn msg(&self) -> Message {
        Message::default().kind(random::<u16>() % 10)
    }
}

impl Module for Host {
    fn at_sim_start(&mut self, _stage: usize) {
        for _ in 0..self.n {
            send(self.msg(), "port");
        }
    }
    fn handle_message(&mut self, _msg: Message) {
        send(self.msg(), "port");
    }
}

#[derive(Default)]
struct Switch;
impl Module for Switch {
    fn handle_message(&mut self, msg: Message) {
        let idx = msg.header().kind as usize;
        send(msg, ("port", idx));
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut ping_pong = c.benchmark_group("network");
    for parallelism in [1, 2, 3, 4, 5] {
        ping_pong.throughput(Throughput::Bytes(parallelism));
        ping_pong.bench_with_input(
            BenchmarkId::from_parameter(parallelism),
            &parallelism,
            |b, &parallelism| {
                b.iter_custom(|iters| {
                    let mut sum = Duration::ZERO;
                    let mut registry = Registry::new()
                        .symbol_fn("Host", |_| Host {
                            n: parallelism as usize,
                        })
                        .symbol::<Switch>("Switch")
                        .with_default_fallback();

                    for _ in 0..iters {
                        let sim = Sim::ndl("simple-routing-traffic.yml", &mut registry).unwrap();
                        let rt = Builder::seeded(123)
                            .quiet()
                            .max_itr(10_000)
                            .build(sim.freeze());
                        let start = Instant::now();
                        let _ = rt.run();
                        sum += start.elapsed()
                    }

                    sum
                })
            },
        );
    }
    ping_pong.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
