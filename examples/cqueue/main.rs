use des::prelude::*;

struct App {
    event_delay: Duration,
    num_events: usize,
}
impl Application for App {
    type EventSet = EvSet;

    fn at_sim_start(rt: &mut Runtime<Self>) {
        let mut delay = Duration::ZERO;
        for _ in 0..rt.app.num_events {
            rt.add_event_in(EvSet {}, delay);
            let offset = random::<f64>() * 2.0 * rt.app.event_delay.as_secs_f64();
            delay += Duration::from_secs_f64(offset);
        }
    }
}

struct EvSet {}
impl EventSet<App> for EvSet {
    fn handle(self, rt: &mut Runtime<App>) {
        rt.add_event_in(EvSet {}, rt.app.event_delay);
        // NOP
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();

    let event_delay: Duration = args
        .iter()
        .find(|v| v.starts_with("delay="))
        .map(|s| s.split("=").collect::<Vec<_>>()[1].parse::<f64>().unwrap())
        .map(Duration::from_secs_f64)
        .unwrap_or(Duration::from_secs(1));

    let num_events: usize = args
        .iter()
        .find(|v| v.starts_with("num="))
        .map(|s| {
            s.split("=").collect::<Vec<_>>()[1]
                .parse::<usize>()
                .unwrap()
        })
        .unwrap_or(400);

    let rt = Runtime::new_with(
        App {
            event_delay,
            num_events,
        },
        RuntimeOptions::seeded(123)
            .include_env()
            .max_time(SimTime::from_duration(Duration::from_secs(100_000))),
    );

    let _ = rt.run();
}
