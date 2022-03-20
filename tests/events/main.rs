use des::*;
use rand::prelude::SliceRandom;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum MyEventSet {
    A(A),
    B(B),
}

impl EventSet<App> for MyEventSet {
    fn handle(self, rt: &mut Runtime<App>) {
        match self {
            Self::A(a) => a.handle(rt),
            Self::B(b) => b.handle(rt),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct A {
    id: usize,
}

impl Event<App> for A {
    fn handle(self, rt: &mut Runtime<App>) {
        rt.app
            .event_list
            .push((SimTime::now(), MyEventSet::A(self)))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventBox {
    time: SimTime,
    events: Vec<MyEventSet>,
}

struct App {
    event_list: Vec<(SimTime, MyEventSet)>,
}

impl Application for App {
    type EventSet = MyEventSet;
}

const N: usize = 100_000;

fn main() {
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

    for eventbox in dispatched {
        let EventBox { time, events } = eventbox;
        for event in events {
            rt.add_event(event, time)
        }
    }

    let (App { event_list }, _) = rt.run().unwrap();
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
                time: time,
                events: vec![event],
            };
        }
    }

    if current_box.events.len() > 0 {
        boxed_list.push(current_box);
    }

    assert_eq!(events.len(), boxed_list.len());

    for (lhs, rhs) in events.into_iter().zip(boxed_list) {
        // assert eq
        assert_eq!(lhs.time, rhs.time);
        assert_eq!(lhs.events.len(), rhs.events.len());

        for l in lhs.events {
            assert!(rhs.events.iter().find(|r| l == **r).is_some())
        }
    }
}

fn random_event(rt: &mut Runtime<App>) -> MyEventSet {
    if rt.random::<bool>() {
        MyEventSet::A(A {
            id: rt.random::<usize>(),
        })
    } else {
        MyEventSet::B(B { id: rt.random() })
    }
}
