use des::prelude::*;
use des_derive::{Module, Network};

#[derive(Debug, Module)]
#[ndl_workspace = "tests/metrics"]
struct Alice {
    core: ModuleCore,
    outvec: OutVec,
}

impl NameableModule for Alice {
    fn named(core: ModuleCore) -> Self {
        Self {
            outvec: OutVec::new("sample_vec".to_string(), Some(core.path().clone()))
                .buffer_max(100)
                .result_dir(String::from("tests/metrics/results")),
            core,
        }
    }
}

impl Module for Alice {
    fn at_sim_start(&mut self, _: usize) {
        self.enable_activity(SimTime::new(1.0))
    }

    fn activity(&mut self) {
        self.outvec.collect(rand::random::<f64>())
    }

    fn at_sim_end(&mut self) {
        self.outvec.finish()
    }

    fn handle_message(&mut self, _: Message) {}
}

#[derive(Debug, Network)]
#[ndl_workspace = "tests/metrics"]
struct Main {}

fn main() {
    Main {}.run_with_options(RuntimeOptions::seeded(123).max_itr(1000));

    let contents =
        std::fs::read_to_string("tests/metrics/results/alice[1]_sample_vec.out").unwrap();

    assert_eq!(contents.chars().filter(|c| *c == '#').count(), 2);
    assert_eq!(contents.lines().count(), 202)
}
