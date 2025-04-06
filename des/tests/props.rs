#![cfg(feature = "net")]

use std::io::ErrorKind;

use des::{net::AsyncFn, prelude::*};
use serial_test::serial;

#[test]
#[serial]
fn parse_props() -> Result<(), RuntimeError> {
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
        AsyncFn::io(|_| async move {
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
        AsyncFn::io(|_| async move {
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

    Builder::seeded(132)
        .max_time(100.0.into())
        .build(sim)
        .run()
        .map(|_| ())
}

#[test]
fn disallow_casting() -> Result<(), RuntimeError> {
    let mut sim = Sim::new(());

    sim.node(
        "alice",
        AsyncFn::io(|_| async move {
            // define prop
            current().prop::<i8>("i8")?.set(123);
            assert_eq!(current().prop::<i8>("i8")?.or_default().get(), 123);
            assert_eq!(
                current().prop::<i32>("i8").unwrap_err().kind(),
                ErrorKind::InvalidInput
            );
            Ok(())
        }),
    );

    Builder::seeded(132)
        .max_time(100.0.into())
        .build(sim)
        .run()
        .map(|_| ())
}
