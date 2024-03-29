use des::{
    net::{
        module::{set_setup_fn, ModuleContext},
        processing::ProcessingElement,
    },
    prelude::*,
    registry,
};

struct A {}

impl Module for A {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        send(Message::new().content(42).build(), "out");
        send(Message::new().content(69).build(), "out");
    }

    fn handle_message(&mut self, msg: Message) {
        let span = ::tracing::span!(::tracing::Level::INFO, "a-recv", age = 2, size = 3);
        let _g = span.enter();
        tracing::info!("recv: {} {}", msg.str(), msg.content::<i32>())
    }
}

#[derive(Default)]
struct PacketCounter {
    count: usize,
}

impl ProcessingElement for PacketCounter {
    fn incoming(&mut self, msg: Message) -> Option<Message> {
        self.count += 1;
        Some(msg)
    }
}

impl Drop for PacketCounter {
    fn drop(&mut self) {
        assert_eq!(self.count, 2);
    }
}

struct B {}

impl Module for B {
    fn stack(&self) -> impl ProcessingElement + 'static
    where
        Self: Sized,
    {
        PacketCounter::default()
    }

    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, msg: Message) {
        send(msg, "out")
    }
}

struct Main;
impl Module for Main {
    fn new() -> Self {
        Self
    }
}

fn empty(_: &ModuleContext) {}

fn main() {
    // Logger::new().set_logger();
    // tracing_subscriber::fmt()
    //     .with_max_level(LevelFilter::TRACE)
    //     .init();

    des::tracing::init();

    // Subscriber::default().init().unwrap();

    set_setup_fn(empty);

    let app = NdlApplication::new("examples/plugin/main.ndl", registry![A, B, Main]).unwrap();
    let app = NetworkApplication::new(app);
    let rt = Builder::new().build(app);
    let _res = rt.run();
}
