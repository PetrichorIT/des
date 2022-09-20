use des::{prelude::*, runtime::ScopedLogger};

#[NdlModule("examples/metrics")]
#[derive(Debug)]
struct Alice {
    outvec: OutVec,
}

impl Module for Alice {
    fn new() -> Self {
        Self {
            outvec: OutVec::new("sample_vec".to_string(), Some(module_path()))
                .buffer_max(100)
                .result_dir(String::from("examples/metrics/results")),
        }
    }

    fn at_sim_start(&mut self, _: usize) {
        schedule_in(Message::new().build(), Duration::from_secs_f64(1.0))
    }

    fn at_sim_end(&mut self) {
        self.outvec.finish()
    }

    fn handle_message(&mut self, _: Message) {
        self.outvec.collect(rand::random::<f64>());
        if SimTime::now() == 42.0 {
            log::trace!("Message");
        }
        schedule_in(Message::new().build(), Duration::from_secs_f64(1.0))
    }
}

#[NdlSubsystem("examples/metrics")]
#[derive(Debug, Default)]
struct Main {}

fn main() {
    ScopedLogger::new()
        .active(true)
        .interal_max_log_level(log::LevelFilter::Warn)
        .finish()
        .expect("Failed to set logger");

    Main::default().run_with_options(RuntimeOptions::seeded(123).max_itr(1000));

    let contents =
        std::fs::read_to_string("examples/metrics/results/alice[1]_sample_vec.out").unwrap();

    assert_eq!(contents.chars().filter(|c| *c == '#').count(), 2);
    assert_eq!(contents.lines().count(), 202)
}
