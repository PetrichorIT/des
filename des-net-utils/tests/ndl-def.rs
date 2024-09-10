use std::collections::HashMap;

use des_net_utils::ndl::def::{
    ConnectionDef, ConnectionEndpointDef, FieldDef, Kardinality, LinkDef,
};
use fxhash::FxHashMap;
use serde_test::{assert_de_tokens, assert_de_tokens_error, assert_tokens, Token};

#[test]
fn test_field_def() {
    assert_tokens(
        &FieldDef {
            ident: "alice".to_string(),
            kardinality: Kardinality::Atom,
        },
        &[Token::Str("alice")],
    );

    assert_tokens(
        &FieldDef {
            ident: "with-dashes".to_string(),
            kardinality: Kardinality::Atom,
        },
        &[Token::Str("with-dashes")],
    );

    assert_tokens(
        &FieldDef {
            ident: "bob".to_string(),
            kardinality: Kardinality::Cluster(3),
        },
        &[Token::Str("bob[3]")],
    );
}

#[test]
fn test_field_def_from_str_error() {
    assert_de_tokens_error::<FieldDef>(
        &[Token::Str("alice[abc]")],
        "invalid digit found in string",
    );
    assert_de_tokens_error::<FieldDef>(
        &[Token::Str("alice[]")],
        "cannot parse integer from empty string",
    );
    assert_de_tokens_error::<FieldDef>(
        &[Token::Str("alice[-12]")],
        "invalid digit found in string",
    );
    assert_de_tokens_error::<FieldDef>(
        &[Token::Str("alice[3.5]")],
        "invalid digit found in string",
    );
    assert_de_tokens_error::<FieldDef>(
        &[Token::Str("alice]")],
        "invalid syntax: expected opening bracket",
    );
}

#[test]
fn test_field_def_serde_error() {
    assert_de_tokens_error::<FieldDef>(
        &[Token::I16(42)],
        "invalid type: integer `42`, expected string",
    );
    assert_de_tokens_error::<FieldDef>(
        &[Token::Map { len: Some(0) }, Token::MapEnd],
        "invalid type: map, expected string",
    );
}

#[test]
fn test_field_def_as_map_key() {
    assert_tokens(
        &HashMap::<FieldDef, String>::from_iter([(
            FieldDef {
                ident: "alice".to_string(),
                kardinality: Kardinality::Atom,
            },
            "Alice".to_string(),
        )]),
        &[
            Token::Map { len: Some(1) },
            Token::Str("alice"),
            Token::Str("Alice"),
            Token::MapEnd,
        ],
    );

    assert_tokens(
        &HashMap::<FieldDef, String>::from_iter([(
            FieldDef {
                ident: "bob".to_string(),
                kardinality: Kardinality::Cluster(3),
            },
            "Bob".to_string(),
        )]),
        &[
            Token::Map { len: Some(1) },
            Token::Str("bob[3]"),
            Token::Str("Bob"),
            Token::MapEnd,
        ],
    );
}

#[test]
fn test_connection_endpoint_def() {
    assert_tokens(
        &ConnectionEndpointDef {
            accessors: vec![FieldDef {
                ident: "alice".to_string(),
                kardinality: Kardinality::Atom,
            }],
        },
        &[Token::Str("alice")],
    );
    assert_tokens(
        &ConnectionEndpointDef {
            accessors: vec![FieldDef {
                ident: "bob".to_string(),
                kardinality: Kardinality::Cluster(3),
            }],
        },
        &[Token::Str("bob[3]")],
    );

    assert_tokens(
        &ConnectionEndpointDef {
            accessors: vec![
                FieldDef {
                    ident: "alice".to_string(),
                    kardinality: Kardinality::Atom,
                },
                FieldDef {
                    ident: "eve".to_string(),
                    kardinality: Kardinality::Atom,
                },
            ],
        },
        &[Token::Str("alice/eve")],
    );

    assert_tokens(
        &ConnectionEndpointDef {
            accessors: vec![
                FieldDef {
                    ident: "bob".to_string(),
                    kardinality: Kardinality::Cluster(3),
                },
                FieldDef {
                    ident: "eve".to_string(),
                    kardinality: Kardinality::Atom,
                },
            ],
        },
        &[Token::Str("bob[3]/eve")],
    );
    assert_tokens(
        &ConnectionEndpointDef {
            accessors: vec![
                FieldDef {
                    ident: "alice".to_string(),
                    kardinality: Kardinality::Atom,
                },
                FieldDef {
                    ident: "bob".to_string(),
                    kardinality: Kardinality::Cluster(3),
                },
            ],
        },
        &[Token::Str("alice/bob[3]")],
    );
    assert_tokens(
        &ConnectionEndpointDef {
            accessors: vec![
                FieldDef {
                    ident: "bob".to_string(),
                    kardinality: Kardinality::Cluster(4),
                },
                FieldDef {
                    ident: "bob".to_string(),
                    kardinality: Kardinality::Cluster(2),
                },
            ],
        },
        &[Token::Str("bob[4]/bob[2]")],
    );
}

#[test]
fn test_connection_without_link_field() {
    assert_tokens(
        &ConnectionDef {
            peers: [
                ConnectionEndpointDef {
                    accessors: vec![FieldDef {
                        ident: "alice".to_string(),
                        kardinality: Kardinality::Atom,
                    }],
                },
                ConnectionEndpointDef {
                    accessors: vec![FieldDef {
                        ident: "bob".to_string(),
                        kardinality: Kardinality::Cluster(3),
                    }],
                },
            ],
            link: None,
        },
        &[
            Token::Struct {
                name: "ConnectionDef",
                len: 1,
            },
            Token::Str("peers"),
            Token::Tuple { len: 2 },
            Token::Str("alice"),
            Token::Str("bob[3]"),
            Token::TupleEnd,
            Token::StructEnd,
        ],
    )
}

#[test]
fn test_connection_with_link_field_null() {
    assert_de_tokens(
        &ConnectionDef {
            peers: [
                ConnectionEndpointDef {
                    accessors: vec![FieldDef {
                        ident: "alice".to_string(),
                        kardinality: Kardinality::Atom,
                    }],
                },
                ConnectionEndpointDef {
                    accessors: vec![FieldDef {
                        ident: "bob".to_string(),
                        kardinality: Kardinality::Cluster(3),
                    }],
                },
            ],
            link: None,
        },
        &[
            Token::Struct {
                name: "ConnectionDef",
                len: 2,
            },
            Token::Str("peers"),
            Token::Tuple { len: 2 },
            Token::Str("alice"),
            Token::Str("bob[3]"),
            Token::TupleEnd,
            Token::Str("link"),
            Token::None,
            Token::StructEnd,
        ],
    );
}

#[test]
fn test_connection_with_link_field_some() {
    assert_de_tokens(
        &ConnectionDef {
            peers: [
                ConnectionEndpointDef {
                    accessors: vec![FieldDef {
                        ident: "alice".to_string(),
                        kardinality: Kardinality::Atom,
                    }],
                },
                ConnectionEndpointDef {
                    accessors: vec![FieldDef {
                        ident: "bob".to_string(),
                        kardinality: Kardinality::Cluster(3),
                    }],
                },
            ],
            link: Some("LAN".to_string()),
        },
        &[
            Token::Struct {
                name: "ConnectionDef",
                len: 2,
            },
            Token::Str("peers"),
            Token::Tuple { len: 2 },
            Token::Str("alice"),
            Token::Str("bob[3]"),
            Token::TupleEnd,
            Token::Str("link"),
            Token::Some,
            Token::Str("LAN"),
            Token::StructEnd,
        ],
    );
}

#[test]
fn test_link_parse_known_keys() {
    assert_tokens(
        &LinkDef {
            latency: 42.5,
            jitter: 0.0,
            bitrate: 80_000,
            other: FxHashMap::default(),
        },
        &[
            // Map, since number of keys is unknown
            Token::Map { len: None },
            Token::Str("latency"),
            Token::F64(42.5),
            Token::Str("jitter"),
            Token::F64(0.0),
            Token::Str("bitrate"),
            Token::I32(80_000),
            Token::MapEnd,
        ],
    )
}

#[test]
fn test_link_parse_use_defaults() {
    assert_de_tokens(
        &LinkDef {
            latency: 42.5,
            jitter: 0.0,
            bitrate: 80_000,
            other: FxHashMap::default(),
        },
        &[
            // Map, since number of keys is unknown
            Token::Map { len: None },
            Token::Str("latency"),
            Token::F64(42.5),
            Token::Str("bitrate"),
            Token::I32(80_000),
            Token::MapEnd,
        ],
    )
}

#[test]
fn test_link_parse_other_keys() {
    assert_de_tokens(
        &LinkDef {
            latency: 0.0,
            jitter: 0.0,
            bitrate: 0,
            other: FxHashMap::from_iter([("other-key".to_string(), "other-value".to_string())]),
        },
        &[
            // Map, since number of keys is unknown
            Token::Map { len: None },
            Token::Str("other-key"),
            Token::Str("other-value"),
            Token::MapEnd,
        ],
    )
}
