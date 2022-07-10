use des::prelude::*;

#[NdlModule("examples/metrics")]
#[derive(Debug)]
struct Alice {
    core: ModuleCore,
    outvec: OutVec,
}

impl NameableModule for Alice {
    fn named(core: ModuleCore) -> Self {
        Self {
            outvec: OutVec::new("sample_vec".to_string(), Some(core.path().clone()))
                .buffer_max(100)
                .result_dir(String::from("examples/metrics/results")),
            core,
        }
    }
}

impl Module for Alice {
    fn at_sim_start(&mut self, _: usize) {
        self.enable_activity(Duration::from(1.0))
    }

    fn activity(&mut self) {
        self.outvec.collect(rand::random::<f64>())
    }

    fn at_sim_end(&mut self) {
        self.outvec.finish()
    }

    fn handle_message(&mut self, _: Message) {}
}

#[NdlSubsystem("examples/metrics")]
#[derive(Debug, Default)]
struct Main {}

fn main() {
    Main::default().run_with_options(RuntimeOptions::seeded(123).max_itr(1000));

    let contents =
        std::fs::read_to_string("examples/metrics/results/alice[1]_sample_vec.out").unwrap();

    assert_eq!(contents.chars().filter(|c| *c == '#').count(), 2);
    assert_eq!(contents.lines().count(), 202)
}
