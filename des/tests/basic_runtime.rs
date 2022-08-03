use des::{prelude::*, runtime::StandardLogger};
use rand::{distributions::Standard, prelude::SliceRandom, Rng};
use serial_test::serial;

/// The Event ste
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum MyEventSet {
    RegisterToRtWithTime(RegisterToRtWithTime),
    B(B),
    RepeatWithDelay(RepeatWithDelay),
}

impl EventSet<App> for MyEventSet {
    fn handle(self, rt: &mut Runtime<App>) {
        match self {
            Self::RegisterToRtWithTime(a) => a.handle(rt),
            Self::B(b) => b.handle(rt),
            Self::RepeatWithDelay(rwd) => rwd.handle(rt),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct RegisterToRtWithTime {
    id: usize,
}

impl Event<App> for RegisterToRtWithTime {
    fn handle(self, rt: &mut Runtime<App>) {
        rt.app
            .event_list
            .push((SimTime::now(), MyEventSet::RegisterToRtWithTime(self)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct B {
    id: usize,
}

impl Event<App> for B {
    fn handle(self, rt: &mut Runtime<App>) {
        rt.app
            .event_list
            .push((SimTime::now(), MyEventSet::B(self)))
    }
}

/// The application
struct App {
    event_list: Vec<(SimTime, MyEventSet)>,
}

impl Application for App {
    type EventSet = MyEventSet;
}

#[test]
#[serial]
fn zero_event_runtime() {
    StandardLogger::active(false);

    let rt = Runtime::<App>::new(App {
        event_list: Vec::new(),
    });

    let res = rt.run();
    assert!(matches!(res, RuntimeResult::EmptySimulation { .. }))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct RepeatWithDelay {
    delay: Duration,
    repeat: usize,
    repeat_limit: usize,
}
impl Event<App> for RepeatWithDelay {
    fn handle(mut self, rt: &mut Runtime<App>) {
        if self.repeat <= self.repeat_limit {
            let delay = self.delay;
            self.repeat += 1;
            rt.add_event_in(MyEventSet::RepeatWithDelay(self), delay)
        }
    }
}

#[test]
#[serial]
fn one_event_runtime() {
    StandardLogger::active(false);

    let mut rt = Runtime::<App>::new(App {
        event_list: Vec::new(),
    });
    rt.add_event(
        MyEventSet::RepeatWithDelay(RepeatWithDelay {
            delay: Duration::new(1, 0),
            repeat: 0,
            repeat_limit: 15,
        }),
        SimTime::ZERO,
    );

    // repeat i = i secs
    // limit (<=) is at 15s thus time limit 16s
    // this means 17 events

    let res = rt.run();
    match res {
        RuntimeResult::Finished {
            time, event_count, ..
        } => {
            assert_eq!(time, SimTime::from_duration(Duration::new(16, 0)));
            assert_eq!(event_count, 17);
        }
        _ => assert!(false, "Runtime should have finished"),
    }
}

#[test]
#[serial]
fn ensure_event_order() {
    StandardLogger::active(false);

    use rand::{rngs::StdRng, SeedableRng};

    let mut id = 0;
    let mut events = Vec::with_capacity(128);
    let mut time = SimTime::ZERO;

    let mut rng = StdRng::seed_from_u64(123);

    for _i in 0..128 {
        time += rng.sample::<f64, Standard>(Standard);
        id += 1;

        events.push((
            MyEventSet::RegisterToRtWithTime(RegisterToRtWithTime { id }),
            time,
        ));
    }

    events.shuffle(&mut rng);

    let mut rt: Runtime<App> = Runtime::new_with(
        App {
            event_list: Vec::with_capacity(128),
        },
        RuntimeOptions::seeded(123),
    );

    for (event, time) in events {
        rt.add_event(event, time);
    }

    match rt.run() {
        RuntimeResult::Finished {
            app,
            time: rt_fin_time,
            event_count,
        } => {
            assert_eq!(rt_fin_time, time);
            assert_eq!(event_count, 128);

            let mut last_id = 0;
            for (_, event) in app.event_list {
                match event {
                    MyEventSet::RegisterToRtWithTime(a) => {
                        assert_eq!(last_id + 1, a.id);
                        last_id += 1;
                    }
                    _ => assert!(false, "Unexpected event"),
                }
            }
        }
        _ => assert!(
            false,
            "Expected runtime to finish after fininte non-replicating event set"
        ),
    }
}

#[test]
#[cfg(not(feature = "cqueue"))]
#[serial]
fn ensure_event_order_same_time() {
    StandardLogger::active(false);

    let one = SimTime::from_duration(Duration::new(1, 0));
    let two = SimTime::from_duration(Duration::new(2, 0));

    let events = vec![
        (
            MyEventSet::RegisterToRtWithTime(RegisterToRtWithTime { id: 1 }),
            SimTime::ZERO,
        ),
        (
            MyEventSet::RegisterToRtWithTime(RegisterToRtWithTime { id: 2 }),
            one,
        ),
        (
            MyEventSet::RegisterToRtWithTime(RegisterToRtWithTime { id: 3 }),
            one,
        ),
        (
            MyEventSet::RegisterToRtWithTime(RegisterToRtWithTime { id: 4 }),
            one,
        ),
        (
            MyEventSet::RegisterToRtWithTime(RegisterToRtWithTime { id: 5 }),
            two,
        ),
    ];

    let mut rt: Runtime<App> = Runtime::new_with(
        App {
            event_list: Vec::with_capacity(32),
        },
        RuntimeOptions::seeded(123),
    );

    for (event, time) in events {
        rt.add_event(event, time);
    }

    match rt.run() {
        RuntimeResult::Finished {
            app,
            time: rt_fin_time,
            event_count,
        } => {
            assert_eq!(rt_fin_time, two);
            assert_eq!(event_count, 5);

            let mut last_id = 0;
            for (_, event) in app.event_list {
                match event {
                    MyEventSet::RegisterToRtWithTime(a) => {
                        assert_eq!(last_id + 1, a.id);
                        last_id += 1;
                    }
                    _ => assert!(false, "Unexpected event"),
                }
            }
        }
        _ => assert!(
            false,
            "Expected runtime to finish after fininte non-replicating event set"
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBox {
    time: SimTime,
    events: Vec<MyEventSet>,
}

const N: usize = 100_000;

#[test]
#[serial]
fn full_test_n_100_000() {
    StandardLogger::active(false);

    let mut rt: Runtime<App> = Runtime::new_with(
        App {
            event_list: Vec::with_capacity(N),
        },
        RuntimeOptions::seeded(123),
    );

    let mut events = Vec::with_capacity(N);

    // create a event set
    let mut t = SimTime::ZERO;
    for _ in 0..N {
        let num_box_elements = rt.random::<usize>() % 100;
        let num_box_elements = if num_box_elements < 5 {
            num_box_elements + 1
        } else {
            1
        };

        let mut boxed = EventBox {
            events: Vec::new(),
            time: t,
        };
        for _ in 0..num_box_elements {
            boxed.events.push(random_event(&mut rt))
        }

        events.push(boxed);

        t += (rt.random::<f64>()).min(0.001);
    }

    let mut dispatched = events.clone();
    dispatched.shuffle(&mut rand::thread_rng());

    let mut c = 0;
    for eventbox in dispatched {
        let EventBox { time, events } = eventbox;
        for event in events {
            rt.add_event(event, time);
            c += 1;
        }
    }

    println!("c := {}", c);

    let (App { event_list }, _, _) = rt.run().unwrap();
    let mut boxed_list = Vec::with_capacity(N);

    let mut current_box = EventBox {
        time: SimTime::ZERO,
        events: Vec::new(),
    };
    for (time, event) in event_list {
        if time == current_box.time {
            current_box.events.push(event);
        } else {
            boxed_list.push(current_box);
            current_box = EventBox {
                time,
                events: vec![event],
            };
        }
    }

    if !current_box.events.is_empty() {
        boxed_list.push(current_box);
    }

    assert_eq!(events.len(), boxed_list.len());

    for (lhs, rhs) in events.into_iter().zip(boxed_list) {
        // assert eq
        assert_eq!(lhs.time, rhs.time);
        assert_eq!(lhs.events.len(), rhs.events.len());

        for l in lhs.events {
            assert!(rhs.events.iter().any(|r| l == *r))
        }
    }
}

fn random_event(rt: &mut Runtime<App>) -> MyEventSet {
    if rt.random::<bool>() {
        MyEventSet::RegisterToRtWithTime(RegisterToRtWithTime {
            id: rt.random::<usize>(),
        })
    } else {
        MyEventSet::B(B { id: rt.random() })
    }
}
