use des::{prelude::*, registry};

#[derive(Default)]
struct A {}

impl Module for A {
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
struct B {}

impl Module for B {
    fn handle_message(&mut self, msg: Message) {
        send(msg, "out")
    }
}

#[derive(Default)]
struct Main;
impl Module for Main {}

fn main() {
    // Logger::new().set_logger();
    // tracing_subscriber::fmt()
    //     .with_max_level(LevelFilter::TRACE)
    //     .init();

    des::tracing::init();

    // Subscriber::default().init().unwrap();

    let app = Sim::ndl("examples/plugin/main.yml", registry![A, B, Main]).unwrap();
    let rt = Builder::new().build(app);
    let _res = rt.run();
}
