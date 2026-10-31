use std::fs::File;

use des::{
    Failure, Sim,
    module::{Prop, current},
    prelude::{Message, Module},
    runtime::handlers::{AsyncHandler, WithContext},
    statistics::time_series::TimeSeries,
};
use serde_norway::Value;
use serial_test::serial;

struct StatisticsAsField {
    series: Prop<TimeSeries<f64>, true>,
}

impl Module for StatisticsAsField {
    fn handle_message(&mut self, msg: des::prelude::Message) {
        self.series.update(|s| s.record(msg.id as f64));
    }
}

#[test]
#[serial]
fn statistics_object_from_non_node_ctx() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        WithContext(|| StatisticsAsField {
            series: current().prop("id").unwrap().or_default(),
        }),
    );
    let gate = sim.gate("alice", "gate");

    let mut builder = sim.seeded(123).build();
    builder.add_message_onto(gate.clone(), Message::default().with_id(1), 1.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(2), 2.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(3), 3.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(2), 4.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(1), 5.0.into());

    let result = builder.run().assert_no_err();
    let col = result
        .app
        .get("alice")
        .unwrap()
        .prop::<TimeSeries<f64>>("id")
        .unwrap()
        .get()
        .unwrap();
    assert_eq!(
        col.values,
        [
            (1.0.into(), 1.0),
            (2.0.into(), 2.0),
            (3.0.into(), 3.0),
            (4.0.into(), 2.0),
            (5.0.into(), 1.0)
        ]
    );

    Ok(())
}

#[test]
#[serial]
fn statistics_report_generated() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        AsyncHandler::once(|_| async move {
            current()
                .prop::<TimeSeries<f64>>("series")
                .unwrap()
                .or_default()
                .update(|r| r.record(2.0));

            current()
                .prop::<f64>("scalar")
                .unwrap()
                .make_statistic()
                .set(42.0);
        }),
    );

    sim.set_output_dir("tests/output".into());

    let _ = sim.seeded(123).build().run().assert_no_err();

    let stats =
        serde_norway::from_reader::<_, Value>(File::open("tests/output/alice.statistics.yml")?)
            .unwrap();
    let stats = stats.as_mapping().unwrap();

    assert_eq!(
        stats["series"].as_mapping().unwrap().iter().next(),
        Some((&0.0.into(), &2.0.into()))
    );
    assert_eq!(stats["scalar"].as_f64().unwrap(), 42.0);

    Ok(())
}
