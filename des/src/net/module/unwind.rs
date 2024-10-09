/// A flag, that defines how the simulation acts if a module panics.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum UnwindBehaviour {
    /// This flag primes the simulation to catch panics.
    /// If a module panics, it will be disabled. The module may only process
    /// data once restarted.
    #[default]
    Catch,
    /// This flag primes the simulation to unwind on panics.
    /// This means, that any panics in module code can panic the entire simulation.
    Unwind,
}
