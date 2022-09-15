use des::{prelude::*, runtime::random};

#[NdlModule("examples/metrics2")]
#[derive(Debug)]
struct HastyModule {
    peak: f64,
}

impl Module for HastyModule {
    fn new() -> Self {
        Self { peak: 0.0 }
    }

    fn at_sim_start(&mut self, _: usize) {
        self.peak = par("peak")
            .as_optional()
            .unwrap_or("10.0".to_string())
            .parse::<f64>()
            .expect("Parse fail");

        schedule_in(Message::new().kind(69).build(), Duration::from_millis(10));
    }

    fn handle_message(&mut self, _: Message) {
        schedule_in(Message::new().kind(69).build(), Duration::from_millis(10));

        let diff = (SimTime::now().as_secs_f64() - self.peak).abs() / 100.0;
        let inv = 1.0 - diff;
        let inv = inv.powi(10);

        let probe = random::<f64>();
        if probe < inv {
            send(
                Message::new()
                    .content(CustomSizeBody::new(8 * 1024, ()))
                    .build(),
                "out",
            )
        }
    }
}

#[NdlModule("examples/metrics2")]
#[derive(Debug)]
struct Collector {}

impl Module for Collector {
    fn new() -> Self {
        Self {}
    }

    fn handle_message(&mut self, _: Message) {}

    fn at_sim_end(&mut self) {
        let chan = gate("out", 0)
            .expect("Expected gate")
            .channel()
            .expect("Expected chan");
        let stats = chan.calculate_stats();

        stats.busy_hist.print();
    }
}

#[NdlModule("examples/metrics2")]
#[derive(Debug)]
struct Consumer {}

impl Module for Consumer {
    fn new() -> Self {
        Self {}
    }
    fn handle_message(&mut self, _: Message) {}
}

#[NdlSubsystem("examples/metrics2")]
#[derive(Debug, Default)]
struct TestCase {}

fn main() {
    let _result = TestCase::default()
        .run_with_options(RuntimeOptions::seeded(123).max_time(SimTime::from(100.0)));
}
