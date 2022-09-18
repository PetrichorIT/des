#![cfg(feature = "net")]

use des::prelude::NetworkRuntime;

const EXAMPLE_NETWORK: &str = "
    netA.*.dnsServer = 1.1.1.1
    netA.s0.ip = 0.0.0.1

    netA.s1.ip = 0.0.0.1

    netA.s1.ipv6 = fe80
";

const EXAMPLE_TYPES: &str = "
    netA.*.text = \"My name\"
    netA.s0.usize = 123

    netA.s1.usize = 420

    netA.s1.isize = -120
";

#[test]
fn non_parse_read() {
    let rt = NetworkRuntime::new(());
    let par = &rt.globals().parameters;

    par.build(EXAMPLE_NETWORK);

    // Case "netA.s0"
    assert_eq!(
        par.get_handle("netA.s0", "dnsServer").as_optional(),
        Some("1.1.1.1".to_string())
    );
    assert_eq!(
        par.get_handle("netA.s0", "ip").as_optional(),
        Some("0.0.0.1".to_string())
    );
    assert_eq!(par.get_handle("netA.s0", "ipv6").as_optional(), None);

    // Case "netA.s1"
    assert_eq!(
        par.get_handle("netA.s1", "dnsServer").as_optional(),
        Some("1.1.1.1".to_string())
    );
    assert_eq!(
        par.get_handle("netA.s1", "ip").as_optional(),
        Some("0.0.0.1".to_string())
    );
    assert_eq!(
        par.get_handle("netA.s1", "ipv6").as_optional(),
        Some("fe80".to_string())
    );

    // Case "netA.other"
    assert_eq!(
        par.get_handle("netA.other", "dnsServer").as_optional(),
        Some("1.1.1.1".to_string())
    );
    assert_eq!(par.get_handle("netA.other", "ip").as_optional(), None);
    assert_eq!(par.get_handle("netA.other", "ipv6").as_optional(), None);
}

#[test]
fn parse_integers() {
    let rt = NetworkRuntime::new(());
    let par = &rt.globals().parameters;

    par.build(EXAMPLE_TYPES);

    // Case "netA.s0"
    assert_eq!(
        *par.get_handle("netA.s0", "text").unwrap(),
        "\"My name\"".to_string()
    );
    assert_eq!(
        par.get_handle("netA.s0", "usize")
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        123
    );
    assert_eq!(par.get_handle("netA.s0", "isize").as_optional(), None);

    // Case "netA.s1"
    assert_eq!(
        *par.get_handle("netA.s1", "text").unwrap(),
        "\"My name\"".to_string()
    );
    assert_eq!(
        par.get_handle("netA.s1", "usize")
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        420
    );
    assert_eq!(
        par.get_handle("netA.s1", "isize")
            .unwrap()
            .parse::<isize>()
            .unwrap(),
        -120
    );

    // Case "netA.other"
    assert_eq!(
        par.get_handle("netA.other", "text").as_optional(),
        Some("\"My name\"".to_string())
    );
    assert_eq!(par.get_handle("netA.other", "usize").as_optional(), None);
    assert_eq!(par.get_handle("netA.other", "isize").as_optional(), None);
}

#[test]
fn parse_strings() {
    let rt = NetworkRuntime::new(());
    let par = &rt.globals().parameters;
    par.build(EXAMPLE_TYPES);

    let handle = par.get_handle("netA.other", "text").unwrap();

    assert_eq!(&*handle, "\"My name\"");
    assert_eq!(handle.parse_string(), "My name".to_string());
}
