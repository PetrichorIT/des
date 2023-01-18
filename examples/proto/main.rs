use des::prelude::*;

#[derive(Debug)]
#[NdlModule("examples/proto")]
struct AppA {}

impl Module for AppA {
    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, _msg: Message) {
        println!("A: [{}] {:?}", SimTime::now(), _msg);
        assert_eq!(SimTime::now(), 1.0);
    }
}

#[derive(Debug)]
#[NdlModule("examples/proto")]
struct AppB {}

impl Module for AppB {
    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, _msg: Message) {
        println!("B: [{}] {:?}", SimTime::now(), _msg);
        assert_eq!(SimTime::now(), 2.0);
    }
}

#[derive(Debug)]
#[NdlModule("examples/proto")]
struct Runner {}

impl Module for Runner {
    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, _msg: Message) {}
}

#[derive(Debug)]
#[NdlModule("examples/proto")]
struct MultiRunner {}

impl Module for MultiRunner {
    fn new() -> Self {
        Self {}
    }

    fn at_sim_start(&mut self, _stage: usize) {
        schedule_at(Message::new().kind(42).build(), 1.0.into());
    }

    fn handle_message(&mut self, mut msg: Message) {
        // println!("M: [{}] {:?}", SimTime::now(), msg);
        if msg.header().kind == 42 {
            let mut dup = msg.dup::<()>();
            dup.header_mut().kind = 123;
            send(dup, ("toAppl", 0));
            // processing_time(Duration::new(1, 0));
            // println!("AAA");
            msg.header_mut().kind = 69;
            send_in(msg, ("toAppl", 1), Duration::from_secs(1));
            schedule_in(Message::new().kind(69).build(), Duration::new(2, 0));
        } else {
            // Send at 1.0 with processing 1.0 and delay 1.0
            assert_eq!(SimTime::now(), 3.0);
        }
    }
}

#[NdlSubsystem("examples/proto")]
#[derive(Debug, Default)]
struct Main();
fn main() {
    Logger::new().try_set_logger().unwrap();
    let app = Main::default().build_rt();

    // println!("{:?}", app.globals().parameters);

    let rt = Runtime::new_with(app, RuntimeOptions::seeded(0x123));
    let (app, _time, _event_count) = rt.run().unwrap();

    let _ = app
        .globals()
        .topology
        .lock()
        .unwrap()
        .write_to_svg("examples/proto/graph");
}
