use rand::distributions::Standard;
use std::{collections::VecDeque, fmt::Debug};

use des::prelude::*;

#[derive(Debug, Clone)]
struct Customer {
    pub arrived: SimTime,
    pub duration: Duration,
}

#[derive(Debug)]
struct Application {
    // Params
    n: usize,
    l: f64,
    m: f64,
    queue: VecDeque<Customer>,
    busy: bool,

    // Metrics
    wait_times: Vec<Duration>,
    busy_time: SimTime,
}

impl Application {
    fn eval(&self, t: SimTime) {
        let busy_perc = self.busy_time / t;

        let avg_wait = self
            .wait_times
            .iter()
            .fold(Duration::ZERO, |acc, &i| acc + i)
            / self.wait_times.len() as u32;

        println!("=== Simulation finished ===");
        println!("l = {} \tm = {}", self.l, self.m);
        println!();
        println!("Finshed at t := {}", t);
        println!("Busy := {}", busy_perc);
        println!("(avg) waittime := {:?}", avg_wait);

        assert!((busy_perc - 0.4996535454771872).abs() < 0.01);
        assert_eq!(avg_wait, Duration::from_secs_f64(1.002135171))
    }
}

impl des::runtime::Application for Application {
    type EventSet = Events;
    type Lifecycle = ();
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum Events {
    ServerDone(ServerDone),
    CustomerArrival(CustomerArrival),
}

impl EventSet<Application> for Events {
    fn handle(self, rt: &mut Runtime<Application>) {
        match self {
            Self::ServerDone(event) => event.handle(rt),
            Self::CustomerArrival(event) => event.handle(rt),
        }
    }
}

#[derive(Debug)]
struct ServerDone {
    started: SimTime,
    _pad: [u8; 300],
}

impl ServerDone {
    fn handle(self, rt: &mut Runtime<Application>) {
        let busy_interval = rt.sim_time() - self.started;
        rt.app.busy_time += busy_interval;

        let customer = rt.app.queue.pop_front();
        match customer {
            Some(customer) => {
                // log wait time
                rt.app.busy = true;
                rt.app.wait_times.push(rt.sim_time() - customer.arrived);
                rt.add_event_in(
                    Events::ServerDone(ServerDone {
                        started: rt.sim_time(),
                        _pad: [0; 300],
                    }),
                    customer.duration,
                )
            }
            None => {
                rt.app.busy = false;
            }
        }
    }
}

#[derive(Debug)]
struct CustomerArrival {
    idx: usize,
}

impl CustomerArrival {
    fn handle(self, rt: &mut Runtime<Application>) {
        if self.idx > rt.app.n {
            return;
        }

        // Gen next event
        let duration = expdist(rt, 1.0 / rt.app.l);
        let next = expdist(rt, 1.0 / rt.app.m);

        let customer = Customer {
            arrived: rt.sim_time(),
            duration: Duration::from_secs_f64(duration),
        };

        if rt.app.busy {
            rt.app.queue.push_back(customer);
        } else {
            rt.app.busy = true;
            rt.app.wait_times.push(Duration::ZERO);
            rt.add_event_in(
                Events::ServerDone(ServerDone {
                    started: rt.sim_time(),
                    _pad: [0; 300],
                }),
                customer.duration,
            );
        }

        rt.add_event_in(
            Events::CustomerArrival(CustomerArrival { idx: self.idx + 1 }),
            Duration::from_secs_f64(next),
        );
    }
}

fn expdist<A: des::runtime::Application>(rt: &mut Runtime<A>, p: f64) -> f64 {
    let x: f64 = rt.rng_sample(Standard);
    x.ln() / -p
}

fn main() {
    let app = Application {
        n: 100_000,
        l: 1.0,
        m: 2.0,

        queue: VecDeque::new(),
        busy: false,

        wait_times: Vec::new(),
        busy_time: SimTime::ZERO,
    };

    let mut rt = Builder::seeded(0x42069).build(app);
    // Create first event
    let l = rt.app.l;
    let dur = Duration::from_secs_f64(expdist(&mut rt, l));
    rt.add_event_in(Events::CustomerArrival(CustomerArrival { idx: 0 }), dur);

    let (app, t_max, _) = rt.run().unwrap();
    app.eval(t_max);
}
