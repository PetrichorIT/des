use std::fmt::Display;
use std::time::Instant;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main};
use rand::prelude::SliceRandom;
use rand::prelude::StdRng;
use rand::Rng;
use rand::SeedableRng;

use cqueue::*;

struct C {
    n: usize,
    t: f64,
}

impl Display for C {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cov = (self.n as f64) * self.t / 2.0;
        write!(f, "{}", cov)
    }
}

fn throughput_by_batch_size(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);
    let mut time = 0.0;
    let mut events = Vec::new();
    for i in 0..10_000 {
        time += rng.sample::<f64, rand::distributions::Open01>(rand::distributions::Open01) / 20.0;
        events.push((time, i))
    }

    events.shuffle(&mut rng);

    let configs = [10, 25, 50, 100, 150, 200, 350, 500];

    let mut queue: CalenderQueue<f64, i32> = CalenderQueue::new_with(CalenderQueueOptions {
        num_buckets: 10,
        bucket_timespan: 20.0,
        min_time: 0.0f64,
        bucket_capacity: 50,
        overflow_capacity: 200,
    });

    let mut group = c.benchmark_group("cqueue::throuput::by_batch_size");
    for c in configs.iter() {
        group.throughput(Throughput::Elements(*c as u64));
        group.bench_with_input(BenchmarkId::from_parameter(c), c, |b, c| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                for i in 0..iters {
                    let i = i as usize;
                    let mut i = i % 10_000;
                    let mut i_end = (i + *c) % 10_000;
                    if i_end < i {
                        i = i_end;
                        i_end = i + *c;
                    }
                    let subsection = &events[i..i_end];

                    // Add elements in all buckets to prevent bucket collpasing
                    for k in 0..10 {
                        let time = 20.0 * k as f64;
                        queue.add(time, k);
                    }

                    for (t, v) in subsection {
                        queue.add(*t, *v);
                    }

                    for _ in subsection {
                        queue.fetch_next();
                    }

                    queue.reset(0.0);
                }
                start.elapsed()
            })
        });
    }
    group.finish()
}

///
/// 100 event throughput at grouped batches with same relative region
///
/// #param = percentatge of the bucket catch area(>100 = overflow)
///
fn throughput_relativ_pos(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);
    let mut time = 0.0;
    let mut events = Vec::new();
    for i in 0..10_000 {
        time += rng.sample::<f64, rand::distributions::Open01>(rand::distributions::Open01) / 20.0;
        events.push((time, i))
    }

    let mut queue: CalenderQueue<f64, i32> = CalenderQueue::new_with(CalenderQueueOptions {
        num_buckets: 10,
        bucket_timespan: 20.0,
        min_time: 0.0f64,
        bucket_capacity: 50,
        overflow_capacity: 200,
    });

    let configs = [
        0.0..10.0,
        10.0..20.0,
        20.0..30.0,
        30.0..40.0,
        40.0..50.0,
        50.0..60.0,
        60.0..70.0,
        70.0..80.0,
        80.0..90.0,
        90.0..100.0,
        100.0..150.0,
    ];

    let subsegements = configs
        .iter()
        .map(|c| {
            let perc = 200.0 * (c.start / 100.0);
            let segment_start = events
                .iter()
                .enumerate()
                .find(|(_, e)| e.0 >= perc)
                .map(|e| e.0)
                .unwrap_or(10_000);

            let perc = 200.0 * (c.end / 100.0);
            let segment_end = events
                .iter()
                .enumerate()
                .find(|(_, e)| e.0 >= perc)
                .map(|e| e.0)
                .unwrap_or(10_000);

            let mut subsegement: Vec<(f64, i32)> =
                events[segment_start..segment_end].iter().cloned().collect();
            subsegement.shuffle(&mut rng);
            subsegement
        })
        .collect::<Vec<Vec<(f64, i32)>>>();

    let mut group = c.benchmark_group("cqueue::throughput::by_insert_position");
    for c in configs.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}", c.1.end)),
            &c,
            |b, c| {
                let subsegement = &subsegements[c.0];
                b.iter_custom(|iters| {
                    queue.clear();
                    let start = Instant::now();

                    for i in 0..iters {
                        let i = i as usize % subsegement.len();

                        // println!("\na) {:?}", queue);

                        // Add elements in all buckets to prevent bucket collpasing
                        for k in 0..10 {
                            let time = 20.0 * k as f64;
                            queue.add(time, k);
                        }

                        for j in i..(i + 100) {
                            let j = j % subsegement.len();
                            let (time, value) = &subsegement[j];
                            queue.add(*time, *value);
                        }

                        // println!("\nb) {:?}", queue);
                        // assert!(queue.len_overflow() > 0);

                        // println!("{} {}", queue.len_first_bucket(), queue.len_overflow());

                        for _j in 0..100 {
                            queue.fetch_next();
                        }

                        queue.reset(0.0)
                    }

                    start.elapsed()
                });
            },
        );
    }
    group.finish()
}

fn throughput_by_parameters(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);

    let mut time = 0.0;
    let mut events = Vec::new();
    for i in 0..10_000 {
        time += rng.sample::<f64, rand::distributions::Open01>(rand::distributions::Open01) / 20.0;
        events.push((time, i))
    }

    events.shuffle(&mut rng);
    let events = &events[..100];
    // 200 s event frame

    let configs = [
        C { n: 5, t: 50.0 },
        C { n: 5, t: 40.0 },
        C { n: 5, t: 30.0 },
        C { n: 5, t: 20.0 },
        C { n: 5, t: 10.0 },
        C { n: 5, t: 2.0 },
        C { n: 5, t: 1.0 },
        C { n: 5, t: 0.2 }, // 1%
    ];

    let mut group = c.benchmark_group("cqueue::throuput::5_buckets");
    for c in configs.iter() {
        // group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(c), c, |b, c| {
            let mut queue: CalenderQueue<f64, i32> =
                CalenderQueue::new_with(CalenderQueueOptions {
                    num_buckets: c.n,
                    bucket_timespan: c.t,
                    min_time: 0.0f64,
                    bucket_capacity: 50,
                    overflow_capacity: 200,
                });

            b.iter(|| {
                queue.reset(0.0);

                for (t, v) in events {
                    queue.add(*t, *v)
                }

                for _ in events {
                    queue.fetch_next();
                }
            });
        });
    }
    group.finish();

    let configs = [
        C { n: 10, t: 25.0 },
        C { n: 10, t: 20.0 },
        C { n: 10, t: 15.0 },
        C { n: 10, t: 10.0 },
        C { n: 10, t: 5.0 },
        C { n: 10, t: 1.0 },
        C { n: 10, t: 0.5 },
        C { n: 10, t: 0.1 },
    ];

    let mut group = c.benchmark_group("cqueue::throuput::10_buckets");
    for c in configs.iter() {
        // group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(c), c, |b, c| {
            let mut queue: CalenderQueue<f64, i32> =
                CalenderQueue::new_with(CalenderQueueOptions {
                    num_buckets: c.n,
                    bucket_timespan: c.t,
                    min_time: 0.0f64,
                    bucket_capacity: 50,
                    overflow_capacity: 200,
                });

            b.iter(|| {
                queue.reset(0.0);
                for (t, v) in events.iter() {
                    queue.add(*t, *v);
                }

                for _ in events {
                    queue.fetch_next();
                }
            });
        });
    }
    group.finish();

    let configs = [
        C { n: 20, t: 12.5 },
        C { n: 20, t: 10.0 },
        C { n: 20, t: 7.5 },
        C { n: 20, t: 5.0 },
        C { n: 20, t: 2.5 },
        C { n: 20, t: 0.5 },
        C { n: 20, t: 0.25 },
        C { n: 20, t: 0.05 },
    ];

    let mut group = c.benchmark_group("cqueue::throuput::20_buckets");
    for c in configs.iter() {
        // group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(c), c, |b, c| {
            let mut queue: CalenderQueue<f64, i32> =
                CalenderQueue::new_with(CalenderQueueOptions {
                    num_buckets: c.n,
                    bucket_timespan: c.t,
                    min_time: 0.0f64,
                    bucket_capacity: 50,
                    overflow_capacity: 200,
                });

            b.iter(|| {
                queue.reset(0.0);
                for (t, v) in events.iter() {
                    queue.add(*t, *v);
                }

                for _ in events {
                    queue.fetch_next();
                }
            });
        });
    }
    group.finish();
}

// criterion_group!(benches, add, add_by_future_pos);
criterion_group!(
    benches,
    throughput_by_batch_size,
    throughput_relativ_pos,
    throughput_by_parameters
);
criterion_main!(benches);
