use crate::{
    runtime::{Application, Profiler, Runtime, RuntimeError},
    time::SimTime,
};

/// The result of a simulation run.
#[derive(Debug)]
pub struct RuntimeResult<A: Application> {
    /// The application instance.
    pub app: A,
    /// The final timestamp within the simulation run.
    pub time: SimTime,
    /// The profiler instance.
    pub profiler: Profiler<A::EventSet>,
    /// Errors which may occur during simulation.
    pub error: Option<RuntimeError>,
}

impl<A: Application> RuntimeResult<A> {
    pub(super) fn new(runtime: Runtime<A>, error: Option<RuntimeError>) -> Self {
        Self {
            time: runtime.sim_time(),
            app: runtime.app,
            profiler: runtime.profiler,
            error,
        }
    }

    /// Asserts that no errors occurred during simulation.
    ///
    /// # Panics
    ///
    /// Panics if any errors occurred during simulation.
    #[track_caller]
    #[must_use]
    pub fn assert_no_err(self) -> Self {
        if let Some(error) = self.error {
            panic!("unwraped with errors: {error}");
        }
        self
    }
}

impl<A: Application> RuntimeResult<A> {
    /// Returns an Err variant if some error occurred during simulation.
    ///
    /// # Errors
    ///
    /// Returns an error if any errors occurred during simulation.
    pub fn as_result(mut self) -> Result<RuntimeResult<A>, RuntimeError> {
        match self.error.take() {
            None => Ok(self),
            Some(error) => Err(error),
        }
    }
}
