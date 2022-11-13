use std::time::{Duration, Instant};

use cqueue::CQueue;
use rand::{distributions::Uniform, rngs::SmallRng, Rng, SeedableRng};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let n = args
        .iter()
        .find(|s| s.starts_with("--cfg-cqueue-n="))
        .map(|s| {
            s.split("=").collect::<Vec<_>>()[1]
                .parse::<usize>()
                .unwrap()
        })
        .unwrap_or(1024);

    let t = Duration::from_secs_f64(
        args.iter()
            .find(|s| s.starts_with("--cfg-cqueue-t="))
            .map(|s| s.split("=").collect::<Vec<_>>()[1].parse::<f64>().unwrap())
            .unwrap_or(0.005),
    );

    let num = args
        .iter()
        .find(|s| s.starts_with("num="))
        .map(|s| {
            s.split("=").collect::<Vec<_>>()[1]
                .parse::<usize>()
                .unwrap()
        })
        .unwrap_or(400);

    let e_delay = args
        .iter()
        .find(|s| s.starts_with("delay="))
        .map(|s| s.split("=").collect::<Vec<_>>()[1].parse::<f64>().unwrap())
        .unwrap_or(1.0);

    let sample = args
        .iter()
        .find(|s| s.starts_with("sample="))
        .map(|s| {
            s.split("=").collect::<Vec<_>>()[1]
                .parse::<usize>()
                .unwrap()
        })
        .unwrap_or(0x12345678);

    let mut cqueue = CQueue::new(n, t);

    // SETUP

    let mut rng = SmallRng::seed_from_u64(sample as u64);

    let mut delay = Duration::ZERO;
    for e in 0..num {
        cqueue.add(delay, e);
        let rng = rng.sample(Uniform::new(0.0, 1.0));
        let offset = rng * 4.0 * e_delay;
        delay += Duration::from_secs_f64(offset);
    }

    // RUN
    let e_delay = Duration::from_secs_f64(e_delay);

    let rng_table: Vec<usize> =
        std::iter::repeat_with(|| rng.sample(Uniform::new(usize::MIN, usize::MAX)))
            .take(num)
            .collect();

    let t0 = Instant::now();
    let mut time = Duration::ZERO;
    let mut c = 0;
    let mut pending_event = Vec::new();

    while c < 100_000_000 {
        let (e, t) = cqueue.fetch_next();
        time = t;

        pending_event.push(e);
        while !pending_event.is_empty() {
            let rng_sample = rng_table[e] % num;
            if rng_sample < pending_event.len() {
                cqueue.add(time + e_delay, pending_event.pop().unwrap())
            } else {
                break;
            }
        }

        // cqueue.add(time + e_delay, e);
        c += 1;
    }

    let _ = time;

    println!("{}", Instant::now().duration_since(t0).as_secs_f64());
}
