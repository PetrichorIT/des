use des::{net::Error, prelude::*, registry};

#[derive(Debug, Default)]
struct A {}

impl A {
    #[tracing::instrument]
    fn method_one(&mut self, value: i32) -> Result<(), Error> {
        self.method_two()?;
        Ok(())
    }

    #[tracing::instrument]
    fn method_two(&mut self) -> Result<(), Error> {
        Err(Error::new(current().path(), des::net::ErrorKind::Other))
    }
}

impl Module for A {
    fn at_sim_start(&mut self, _stage: usize) {
        let _ = send(Message::default().with_content(42), "out");
        let _ = send(Message::default().with_content(69), "out");
    }

    fn handle_message(&mut self, msg: Message) {
        let span = ::tracing::span!(::tracing::Level::INFO, "a-recv", age = 2, size = 3);
        let _g = span.enter();
        tracing::info!("recv: {} {}", msg, msg.body.content::<i32>());
    }

    fn at_sim_end(&mut self) -> Result<(), RuntimeError> {
        self.method_one(123)?;
        Ok(())
    }
}

#[derive(Default)]
struct B {}

impl Module for B {
    fn handle_message(&mut self, msg: Message) {
        let _ = send(msg, "out");
    }
}

#[derive(Default)]
struct Main;
impl Module for Main {}

fn main() -> Result<(), RuntimeError> {
    // Logger::new().set_logger();
    // tracing_subscriber::fmt()
    //     .with_max_level(LevelFilter::TRACE)
    //     .init();

    des::tracing::init();

    // Subscriber::default().init().unwrap();

    let app = Sim::ndl("examples/plugin/main.yml", registry![A, B, Main]).unwrap();
    let rt = Builder::new().build(app.freeze());
    rt.run().map(|_| ())
}
