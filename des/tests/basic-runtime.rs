use des::{prelude::*, runtime::RuntimeLimit};
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

impl RegisterToRtWithTime {
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

impl B {
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
    type Lifecycle = ();
}

#[test]
#[serial]
fn zero_event_runtime() {
    let rt = Builder::seeded(123).build(App {
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
impl RepeatWithDelay {
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
    let mut rt = Builder::new().build(App {
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
        RuntimeResult::Finished { time, profiler, .. } => {
            assert_eq!(time, SimTime::from_duration(Duration::new(16, 0)));
            assert_eq!(profiler.event_count, 17);
        }
        _ => panic!("Runtime should have finished"),
    }
}

#[test]
#[serial]
fn ensure_event_order() {
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

    let mut rt: Runtime<App> = Builder::seeded(123).build(App {
        event_list: Vec::with_capacity(128),
    });

    for (event, time) in events {
        rt.add_event(event, time);
    }

    match rt.run() {
        RuntimeResult::Finished {
            app,
            time: rt_fin_time,
            profiler,
        } => {
            assert_eq!(rt_fin_time, time);
            assert_eq!(profiler.event_count, 128);

            let mut last_id = 0;
            for (_, event) in app.event_list {
                match event {
                    MyEventSet::RegisterToRtWithTime(a) => {
                        assert_eq!(last_id + 1, a.id);
                        last_id += 1;
                    }
                    _ => panic!("Unexpected event"),
                }
            }
        }
        _ => panic!("Expected runtime to finish after fininte non-replicating event set"),
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

    let mut rt: Runtime<App> = Builder::seeded(123).build(App {
        event_list: Vec::with_capacity(32),
    });

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
                    _ => panic!("Unexpected event"),
                }
            }
        }
        _ => panic!("Expected runtime to finish after fininte non-replicating event set"),
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
    let mut rt: Runtime<App> = Builder::seeded(123).build(App {
        event_list: Vec::with_capacity(N),
    });

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

struct DeferredApplication {
    started: bool,
    ended: bool,
}
impl EventLifecycle for DeferredApplication {
    fn at_sim_start(rt: &mut Runtime<Self>) {
        rt.app.started = true;
    }
    fn at_sim_end(rt: &mut Runtime<Self>) {
        rt.app.ended = true;
    }
}
impl Application for DeferredApplication {
    type Lifecycle = Self;
    type EventSet = DeferredES;
}

struct DeferredES;
impl EventSet<DeferredApplication> for DeferredES {
    fn handle(self, _rt: &mut Runtime<DeferredApplication>) {}
}

#[test]
#[serial]
fn deferred_sim_start() {
    let app = DeferredApplication {
        started: false,
        ended: false,
    };
    let rt = Builder::seeded(123).build(app);

    assert_eq!(rt.app.started, false);
    assert_eq!(rt.app.ended, false);

    let app = match rt.run() {
        RuntimeResult::EmptySimulation { app } => app,
        _ => panic!("Which events?"),
    };

    assert_eq!(app.started, true);
    assert_eq!(app.ended, true);
}

struct CustomStartApp;
impl Application for CustomStartApp {
    type EventSet = CustomStartEvent;
    type Lifecycle = Self;
}

struct CustomStartEvent;
impl EventSet<CustomStartApp> for CustomStartEvent {
    fn handle(self, _: &mut Runtime<CustomStartApp>) {}
}

impl EventLifecycle for CustomStartApp {
    fn at_sim_start(_: &mut Runtime<Self>) {
        assert_eq!(SimTime::now(), 42.0);
    }
}

#[test]
#[serial]
fn custom_start_time() {
    let _ = Builder::new()
        .quiet()
        .start_time(42.0.into())
        .limit(RuntimeLimit::EventCount(10))
        .build(CustomStartApp)
        .run();
}

struct PausableApp;
impl Application for PausableApp {
    type EventSet = PausableAppEvent;
    type Lifecycle = PausableApp;
}

impl EventLifecycle for PausableApp {
    fn at_sim_start(runtime: &mut Runtime<Self>)
    where
        Self: Application,
    {
        runtime.add_event(PausableAppEvent(0), SimTime::ZERO);
    }
}

struct PausableAppEvent(usize);
impl EventSet<PausableApp> for PausableAppEvent {
    fn handle(mut self, runtime: &mut Runtime<PausableApp>) {
        self.0 += 1;
        runtime.add_event_in(self, Duration::from_secs(1))
    }
}

#[test]
#[serial]
fn pausable_app() {
    let mut sim = Builder::new()
        .quiet()
        .limit(RuntimeLimit::EventCount(1000))
        .build(PausableApp);

    sim.start();

    assert_eq!(sim.num_events_dispatched(), 0);

    sim.dispatch_n_events(10);
    assert_eq!(sim.num_events_dispatched(), 10);

    sim.dispatch_events_until(42.0.into());
    assert_eq!(sim.num_events_dispatched(), 43); // 0...42 ??

    sim.dispatch_all();
    assert_eq!(sim.num_events_dispatched() + 1, sim.num_events_scheduled());
    assert_eq!(sim.num_events_dispatched(), 1000);
}
