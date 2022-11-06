use std::time::Duration;

use cqueue::CQueue;

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

    let mut cqueue = CQueue::new(n, t);

    // SETUP

    let rng = [0.123, 1.89123, 1.2223, 0.878, 0.4657, 1.123, 1.645];

    let mut delay = Duration::ZERO;
    for e in 0..num {
        cqueue.add(delay, e);
        let offset = rng[e % rng.len()] * 2.0 * e_delay;
        delay += Duration::from_secs_f64(offset);
    }

    // RUN

    let mut time = Duration::ZERO;
    let mut c = 0;
    while time < Duration::from_secs(100_000) && !cqueue.is_empty() {
        let (e, t) = cqueue.fetch_next();
        time = t;
        cqueue.add(time + Duration::from_secs_f64(e_delay), e);
        c += 1;
    }
    println!(
        "Event count: {} at {}s with remaining {}",
        c,
        time.as_secs(),
        cqueue.len()
    );
}
