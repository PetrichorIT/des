use std::{hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use des::{
    runtime::{Application, Builder, Event as EventSet, EventLifecycle},
    time::SimTime,
};

fn sim_lock_aquire() {
    struct App;
    impl Application for App {
        type EventSet = ();
        type Lifecycle = ();
    }

    let _ = Builder::new().max_itr(1000).build(black_box(App));
}

fn ping_pong_application(n: usize) {
    struct App {
        n: usize,
    }
    impl Application for App {
        type EventSet = Event;
        type Lifecycle = Self;
    }

    enum Event {
        Ping,
        Pong,
    }

    impl EventLifecycle for App {
        fn at_sim_start(runtime: &mut des::prelude::Runtime<Self>) {
            for _ in 0..runtime.app.n {
                runtime.add_event(Event::Ping, SimTime::ZERO);
            }
        }
    }

    impl EventSet<App> for Event {
        fn handle(self, runtime: &mut des::prelude::Runtime<App>) {
            runtime.add_event_in(
                match self {
                    Self::Ping => Self::Pong,
                    Self::Pong => Self::Ping,
                },
                Duration::from_secs(1),
            );
        }
    }

    let _ = Builder::new()
        .max_itr(10_000)
        .quiet()
        .build(App { n })
        .run();
}

fn unevenly_spaced_events(n: usize) {
    struct App {
        n: usize,
    }
    impl Application for App {
        type EventSet = Event;
        type Lifecycle = Self;
    }

    struct Event(usize);

    impl EventLifecycle for App {
        fn at_sim_start(runtime: &mut des::prelude::Runtime<Self>) {
            for _ in 0..runtime.app.n {
                runtime.add_event(Event(2), SimTime::ZERO);
            }
        }
    }

    impl EventSet<App> for Event {
        fn handle(self, runtime: &mut des::prelude::Runtime<App>) {
            let value = [0.1, 0.5, 1.0, 5.0, 10.0][self.0 % 5];
            runtime.add_event_in(Event(self.0 + 1), Duration::from_secs_f64(value));
        }
    }

    let _ = Builder::new()
        .max_itr(10_000)
        .quiet()
        .build(App { n })
        .run();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("sim-lock-aquire", |b| b.iter(|| sim_lock_aquire()));

    let mut ping_pong = c.benchmark_group("ping-pong");
    for parallelism in [1, 10, 100, 1000] {
        ping_pong.throughput(Throughput::Bytes(parallelism));
        ping_pong.bench_with_input(
            BenchmarkId::from_parameter(parallelism),
            &parallelism,
            |b, &parallelism| b.iter(|| ping_pong_application(parallelism as usize)),
        );
    }
    ping_pong.finish();

    let mut ping_pong = c.benchmark_group("uneven-events");
    for parallelism in [1, 10, 100, 1000] {
        ping_pong.throughput(Throughput::Bytes(parallelism));
        ping_pong.bench_with_input(
            BenchmarkId::from_parameter(parallelism),
            &parallelism,
            |b, &parallelism| b.iter(|| unevenly_spaced_events(parallelism as usize)),
        );
    }
    ping_pong.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
