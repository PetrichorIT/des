#![cfg(feature = "net")]
#![allow(unused)]
use des::{prelude::*, registry};
use serial_test::serial;

#[macro_use]
mod common;

struct TopLevelModule {
    state: u32,
}

impl Module for TopLevelModule {
    fn new() -> Self {
        println!("### TopLevelModule::new");
        Self { state: 42 }
    }
}

struct MidLevelModule {
    state: i64,
}

impl Module for MidLevelModule {
    fn new() -> Self {
        // gates loaded
        assert!(gate("in", 0).is_some());
        assert!(gate("out", 0).is_some());

        // child loaded
        let child = child("child");
        assert!(child.is_ok());
        assert!(child
            .as_ref()
            .unwrap()
            .try_as_ref::<LowLevelModule>()
            .is_some());
        assert_eq!(
            child.as_ref().unwrap().as_ref::<LowLevelModule>().state,
            0u8
        );

        // parent not loaded yet
        let parent = parent();
        assert!(parent.is_err());
        assert!(matches!(
            parent.unwrap_err(),
            ModuleReferencingError::NotYetInitalized(_)
        ));

        Self { state: -69 }
    }

    fn at_sim_start(&mut self, _stage: usize) {
        let parent = parent();
        assert!(parent.is_ok());
        assert!(parent
            .as_ref()
            .unwrap()
            .try_as_ref::<TopLevelModule>()
            .is_some());
    }
}

struct LowLevelModule {
    state: u8,
}

impl Module for LowLevelModule {
    fn new() -> Self {
        assert!(gate("in", 0).is_some());
        assert!(gate("out", 0).is_some());

        Self { state: 0 }
    }
}

#[test]
#[serial]
fn load_order() {
    let rt = NetworkRuntime::new(
        NdlApplication::new(
            "tests/load_order.ndl",
            registry![TopLevelModule, MidLevelModule, LowLevelModule],
        )
        .map_err(|e| println!("{e}"))
        .unwrap(),
    );
    let rt = Runtime::new(rt);

    let _ = rt.run();
}
