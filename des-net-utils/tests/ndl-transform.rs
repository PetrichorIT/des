use des_net_utils::ndl::{
    def::{Def, FieldDef, Kardinality},
    error::Error,
    transform,
};
use serde_json::json;

#[test]
fn test_unknown_entry() {
    let def: Def = serde_json::from_value(json!({
        "entry": "B",
        "modules": {
            "A": {}
        }
    }))
    .unwrap();

    assert_eq!(transform(&def), Err(Error::UnknownModule("B".to_string())));
}

#[test]
fn test_unresolvable_dependencies_cycle() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "submodules": {
                    "b": "B"
                }
            },
            "B": {
                "submodules": {
                    "a": "A"
                }
            }
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::UnresolvableDependency(vec![
            "A".to_string(),
            "B".to_string()
        ]))
    );
}

// TODO: needs clearer error message
#[test]
fn test_unresolvable_dependencies_unknown_typ() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "submodules": {
                    "b": "B"
                }
            },
            "B": {
                "submodules": {
                    "c": "C"
                }
            }
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::UnresolvableDependency(vec![
            "A".to_string(),
            "B".to_string()
        ]))
    );
}

#[test]
fn test_invalid_gate() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates": ["alice", "bob[3]", "eve[0]"]
            },
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::InvalidGate("A".to_string(), "eve".to_string()))
    );
}

#[test]
fn test_invalid_submodule() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "submodules": {
                    "alice": "B",
                    "bob[3]": "B",
                    "eve[0]": "B"
                }
            },
            "B": {}
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::InvalidSubmodule("A".to_string(), "eve".to_string()))
    );
}

#[test]
fn test_invalid_connection_submodule() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates":[ "port"],
                "submodules": {
                    "alice": "B",
                },
                "connections":  [
                    {
                        "peers": ["bob/port", "port"]
                    }
                ]
            },
            "B": {
                "gates": ["port"]
            }
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::UnknownSubmoduleInConnection(
            0,
            FieldDef {
                ident: "bob".to_string(),
                kardinality: Kardinality::Atom
            }
        ))
    );
}

#[test]
fn test_invalid_connection_gate() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates":[ "port"],
                "submodules": {
                    "alice": "B",
                },
                "connections":  [
                    {
                        "peers": ["alice/port", "pooooort"]
                    }
                ]
            },
            "B": {
                "gates": ["port"]
            }
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::UnknownGateInConnection(
            0,
            FieldDef {
                ident: "pooooort".to_string(),
                kardinality: Kardinality::Atom
            }
        ))
    );

    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates":[ "port"],
                "submodules": {
                    "alice": "B",
                },
                "connections":  [
                    {
                        "peers": ["alice/pooooort", "oprt"]
                    }
                ]
            },
            "B": {
                "gates": ["port"]
            }
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::UnknownGateInConnection(
            0,
            FieldDef {
                ident: "pooooort".to_string(),
                kardinality: Kardinality::Atom
            }
        ))
    );
}

#[test]
fn test_connection_invalid_access_out_of_bounds() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates":[ "port[3]"],
                "submodules": {
                    "alice[3]": "B",
                },
                "connections":  [
                    {
                        "peers": ["alice[4]/port", "port[2]"]
                    }
                ]
            },
            "B": {
                "gates": ["port"]
            }
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::ConnectionIndexOutOfBounds(
            0,
            FieldDef {
                ident: "alice".to_string(),
                kardinality: Kardinality::Cluster(4)
            }
        ))
    );
}

#[test]
fn test_connection_invalid_access_non_indexable() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates":[ "port[3]"],
                "submodules": {
                    "alice": "B",
                },
                "connections":  [
                    {
                        "peers": ["alice[4]/port", "port[2]"]
                    }
                ]
            },
            "B": {
                "gates": ["port"]
            }
        }
    }))
    .unwrap();

    assert_eq!(
        transform(&def),
        Err(Error::ConnectionIndexOutOfBounds(
            0,
            FieldDef {
                ident: "alice".to_string(),
                kardinality: Kardinality::Cluster(4)
            }
        ))
    );
}

#[test]
fn test_connection_unequal_peers() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates":[ "port"],
                "submodules": {
                    "alice": "B",
                },
                "connections":  [
                    {
                        "peers": ["alice/port", "port"]
                    }
                ]
            },
            "B": {
                "gates": ["port[3]"]
            }
        }
    }))
    .unwrap();

    assert_eq!(transform(&def), Err(Error::UnequalPeers(0, 3, 1)));
}

#[test]
fn test_unknown_link() {
    let def: Def = serde_json::from_value(json!({
        "entry": "A",
        "modules": {
            "A": {
                "gates":[ "port"],
                "submodules": {
                    "alice": "B",
                },
                "connections":  [
                    {
                        "peers": ["alice/port", "port"],
                        "link": "LAN"
                    }
                ]
            },
            "B": {
                "gates": ["port"]
            }
        }
    }))
    .unwrap();

    assert_eq!(transform(&def), Err(Error::UnknownLink("LAN".to_string())));
}
