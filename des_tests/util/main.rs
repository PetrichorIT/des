#[allow(unused)]
use des_core::*;
use util_macros::EventSuperstructure;

struct App;
impl Application for App {
    type EventSuperstructure = Events;
}

#[derive(EventSuperstructure)]
enum Events {
    EventA(EventA),
    EventB(EventB),
}

struct EventA();

impl Event<App> for EventA {
    fn handle(self, _rt: &mut Runtime<App>) {}
}

struct EventB();

impl Event<App> for EventB {
    fn handle(self, _rt: &mut Runtime<App>) {}
}

fn main() {
    let _ev: Events = EventB().into();

    // let a: <EventA as Event<App>>::EventSuperstructure = Events::EventA(todo!());
}
