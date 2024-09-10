use std::collections::HashMap;

use des_networks::ndl::{def::*, transform};
use fxhash::FxHashMap;

#[test]
fn module_order() {
    let def = Def {
        entry: "A".to_string(),
        links: FxHashMap::default(),
        modules: HashMap::from_iter([
            (
                "A".to_string(),
                ModuleDef {
                    parent: None,
                    submodules: HashMap::from_iter([(
                        FieldDef {
                            ident: "b".to_string(),
                            kardinality: Kardinality::Atom,
                        },
                        "B".to_string(),
                    )]),
                    gates: Vec::new(),
                    connections: Vec::new(),
                },
            ),
            (
                "B".to_string(),
                ModuleDef {
                    parent: None,
                    submodules: HashMap::from_iter([
                        (
                            FieldDef {
                                ident: "c".to_string(),
                                kardinality: Kardinality::Atom,
                            },
                            "C".to_string(),
                        ),
                        (
                            FieldDef {
                                ident: "d".to_string(),
                                kardinality: Kardinality::Cluster(3),
                            },
                            "D".to_string(),
                        ),
                    ]),
                    gates: vec![GateDef {
                        ident: "port".to_string(),
                        kardinality: Kardinality::Cluster(3),
                    }],
                    connections: vec![ConnectionDef {
                        peers: [
                            ConnectionEndpointDef {
                                accessors: vec![FieldDef {
                                    ident: "port".to_string(),
                                    kardinality: Kardinality::Atom,
                                }],
                            },
                            ConnectionEndpointDef {
                                accessors: vec![
                                    FieldDef {
                                        ident: "d".to_string(),
                                        kardinality: Kardinality::Atom,
                                    },
                                    FieldDef {
                                        ident: "plug".to_string(),
                                        kardinality: Kardinality::Atom,
                                    },
                                ],
                            },
                        ],
                        link: None,
                    }],
                },
            ),
            (
                "C".to_string(),
                ModuleDef {
                    parent: Some("D".to_string()),
                    submodules: FxHashMap::default(),
                    gates: Vec::new(),
                    connections: Vec::new(),
                },
            ),
            (
                "D".to_string(),
                ModuleDef {
                    parent: None,
                    submodules: FxHashMap::default(),
                    gates: vec![GateDef {
                        ident: "plug".to_string(),
                        kardinality: Kardinality::Atom,
                    }],
                    connections: Vec::new(),
                },
            ),
        ]),
    };

    dbg!(transform(&def).unwrap());
}
