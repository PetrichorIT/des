use des::{Failure, prelude::*, runtime::handlers::AsyncHandler, time::sleep_until};
use serde::{Deserialize, Serialize};
use serde_norway::{Number, Value};
use serial_test::serial;

#[test]
#[serial]
fn parse_props() -> Result<(), Failure> {
    let mut sim = Sim::new(());

    sim.include_cfg(
        "preset.number: 123\n\
        preset.number_neg: -371\n\
        preset.string: Non terminated String\n\
        preset.string_t: \"This is a nice, : string\"\n\
        preset.bool: true\n\
        preset.v4: '192.168.2.101'\n\
        preset.v6: fe80::132\n\
        ",
    );

    sim.node(
        "preset",
        AsyncHandler::io(|_| async move {
            assert_eq!(current().prop::<usize>("number")?.or_default().get(), 123);
            assert_eq!(
                current().prop::<i16>("number_neg")?.or_default().get(),
                -371
            );
            assert_eq!(
                current().prop::<String>("string")?.or_default().get(),
                "Non terminated String".to_string()
            );
            assert_eq!(
                current().prop::<String>("string_t")?.or_default().get(),
                "This is a nice, : string".to_string()
            );
            assert_eq!(current().prop::<bool>("bool")?.or_default().get(), true);
            assert_eq!(
                current().prop::<Option<Ipv4Addr>>("v4")?.or_default().get(),
                Some(Ipv4Addr::new(192, 168, 2, 101))
            );
            assert_eq!(
                current().prop::<Option<Ipv6Addr>>("v6")?.or_default().get(),
                Some(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x132))
            );
            Ok(())
        }),
    );

    sim.include_cfg(
        "\
        list.one: ['1.1.1.1']\n\
        list.more_delmitied: [1,2,3,4,5,6]\n\
        list.more_no_trailing: [1,2,3,4,5,6]\n\
        ",
    );

    sim.node(
        "list",
        AsyncHandler::io(|_| async move {
            assert_eq!(
                current().prop::<Vec<Ipv4Addr>>("one")?.or_default().get(),
                vec![Ipv4Addr::new(1, 1, 1, 1)]
            );
            assert_eq!(
                current()
                    .prop::<Vec<usize>>("more_delmitied")?
                    .or_default()
                    .get(),
                vec![1, 2, 3, 4, 5, 6]
            );
            assert_eq!(
                current()
                    .prop::<Vec<u8>>("more_no_trailing")?
                    .or_default()
                    .get(),
                vec![1, 2, 3, 4, 5, 6]
            );

            Ok(())
        }),
    );

    sim.seeded(132)
        .max_time(100.0.into())
        .build()
        .run()
        .into_result()
        .map(|_| ())
}

#[test]
#[serial]
fn disallow_casting() -> Result<(), Failure> {
    let mut sim = Sim::new(());

    sim.node(
        "alice",
        AsyncHandler::io(|_| async move {
            // define prop
            current().prop::<i8>("i8")?.set(123);
            assert_eq!(current().prop::<i8>("i8")?.or_default().get(), 123);
            // assert_eq!(
            //     current().prop::<i32>("i8").unwrap_err().kind,
            //     ErrorKind::InvalidInput
            // ); TODO make errors more expresive
            Ok(())
        }),
    );

    sim.seeded(132)
        .max_time(100.0.into())
        .build()
        .run()
        .into_result()
        .map(|_| ())
}

#[test]
#[serial]
fn prop_tracer() -> Result<(), Failure> {
    let mut sim = Sim::new(());

    sim.node(
        "alice",
        AsyncHandler::io(|_| async move {
            let mut prop = current().prop::<usize>("usize")?.or_default();
            prop.set(0);
            prop.add_tracer("");

            sleep_until(1.0.into()).await;
            prop.set(1);

            sleep_until(2.0.into()).await;
            prop.set(2);

            sleep_until(3.0.into()).await;
            prop.set(2);

            sleep_until(4.0.into()).await;
            prop.update(|v| {
                *v = 3;
                *v = 4;
            });

            let tracers = prop.tracers().remove(0);
            assert_eq!(
                tracers.history,
                [
                    (0.0.into(), Value::Number(Number::from(0))),
                    (1.0.into(), Value::Number(Number::from(1))),
                    (2.0.into(), Value::Number(Number::from(2))),
                    (4.0.into(), Value::Number(Number::from(4)))
                ]
            );

            Ok(())
        })
        .require_join(),
    );

    sim.seeded(132)
        .max_time(100.0.into())
        .build()
        .run()
        .into_result()
        .map(|_| ())
}

#[test]
#[serial]
fn prop_tracer_subvalue() -> Result<(), Failure> {
    let mut sim = Sim::new(());

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct SuperValue {
        number: usize,
        list: Vec<usize>,
    }

    sim.node(
        "alice",
        AsyncHandler::io(|_| async move {
            let mut prop = current().prop::<SuperValue>("usize")?.or_default();
            prop.add_tracer("number");

            prop.set(SuperValue {
                number: 1,
                list: vec![1, 2, 3],
            });
            prop.add_tracer("list.0");

            sleep_until(1.0.into()).await;
            prop.update(|v| v.number = 1);

            sleep_until(2.0.into()).await;
            prop.update(|v| v.list[0] = 11);

            sleep_until(3.0.into()).await;
            prop.update(|v| {
                v.number = 3;
                v.list[0] = 111;
            });

            sleep_until(4.0.into()).await;
            prop.update(|v| v.number = 4);

            let tracers = prop.tracers();
            assert_eq!(tracers[0].selector, "number");
            assert_eq!(
                tracers[0].history,
                [
                    (0.0.into(), Value::Number(Number::from(0))),
                    (0.0.into(), Value::Number(Number::from(1))),
                    (3.0.into(), Value::Number(Number::from(3))),
                    (4.0.into(), Value::Number(Number::from(4))),
                ]
            );

            assert_eq!(tracers[1].selector, "list.0");
            assert_eq!(
                tracers[1].history,
                [
                    (0.0.into(), Value::Number(Number::from(1))),
                    (2.0.into(), Value::Number(Number::from(11))),
                    (3.0.into(), Value::Number(Number::from(111))),
                ]
            );

            Ok(())
        })
        .require_join(),
    );

    sim.seeded(132)
        .max_time(100.0.into())
        .build()
        .run()
        .into_result()
        .map(|_| ())
}
