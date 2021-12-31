use des_core::*;
use util_macros::EventSuperstructure;

struct App;

#[derive(EventSuperstructure)]
enum Events {
    #[allow(unused)]
    EventA(EventA),
    #[allow(unused)]
    EventB(EventB),
}

struct EventA();

impl Event<App> for EventA {
    type EventSuperstructure = Events;

    fn handle(self, _rt: &mut Runtime<App, Self::EventSuperstructure>) {}
}

struct EventB();

impl Event<App> for EventB {
    type EventSuperstructure = Events;

    fn handle(self, _rt: &mut Runtime<App, Self::EventSuperstructure>) {}
}

fn main() {

    // let a: <EventA as Event<App>>::EventSuperstructure = Events::EventA(todo!());
}
