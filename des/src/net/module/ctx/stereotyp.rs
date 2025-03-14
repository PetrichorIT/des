/// A stereotyp that defines a nodes behaviour on startup, shutdown or panic.
///
/// The lifecycle of a node is defined as follows:
/// 1. A node is created (potentially from non-sim context)
/// 2. A node is started (`at_sim_start`)
/// 3. A node runs.
/// 4. The software panics (shutdown/restart is also treated as a panic???)
///  1. Node is either restarted or dropped
///  2. Children are droped if flag is set
///  3. Parent is informed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Stereotyp {
    /// TODO
    pub on_panic_catch: bool,
    /// TODO
    pub on_panic_drop: bool,
    /// TODO
    pub on_panic_restart: bool,
    /// TODO
    pub on_panic_drop_submodules: bool,
    /// TODO
    pub on_panic_inform_parent: bool,
}

impl Stereotyp {
    /// TODO
    pub const HOST: Stereotyp = Stereotyp {
        on_panic_catch: false,
        on_panic_drop: false,
        on_panic_restart: true,

        on_panic_drop_submodules: true,
        on_panic_inform_parent: false,
    };

    /// TODO
    pub const SUBPROCESS: Stereotyp = Stereotyp {
        on_panic_catch: true,
        on_panic_drop: true,
        on_panic_restart: false,

        on_panic_drop_submodules: true,
        on_panic_inform_parent: true,
    };
}

impl Default for Stereotyp {
    fn default() -> Self {
        Self::HOST
    }
}
