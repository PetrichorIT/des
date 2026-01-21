use des::{
    Failure, Sim,
    prelude::{Message, Module},
    statistics::time_series::TimeSeries,
};
use serial_test::serial;

struct StatisticsAsField {
    series: TimeSeries,
}

impl Module for StatisticsAsField {
    fn handle_message(&mut self, msg: des::prelude::Message) {
        self.series.record(msg.id as f64);
    }
}

#[test]
#[serial]
fn statistics_object_from_non_node_ctx() -> Result<(), Failure> {
    let mut sim = Sim::new(());
    sim.node(
        "alice",
        StatisticsAsField {
            series: TimeSeries::new("id"),
        },
    );
    let gate = sim.gate("alice", "gate");

    let mut builder = sim.seeded(123).build();
    builder.add_message_onto(gate.clone(), Message::default().with_id(1), 1.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(2), 2.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(3), 3.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(2), 4.0.into());
    builder.add_message_onto(gate.clone(), Message::default().with_id(1), 5.0.into());

    let result = builder.run().assert_no_err();
    let ((path, key), col) = result
        .app
        .statistics
        .collectors::<TimeSeries>()
        .next()
        .unwrap();
    assert_eq!(path, "alice");
    assert_eq!(key, "id");
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
