use crate::{Failure, Sim, SimLifecycle, time::SimTime};

/// The result of a simulation run.
#[derive(Debug)]
pub struct RuntimeResult<A> {
    /// The application instance.
    pub app: Sim<A>,
    /// The final timestamp within the simulation run.
    pub time: SimTime,
    /// Errors which may occur during simulation.
    pub error: Option<Failure>,
}

impl<A: SimLifecycle> RuntimeResult<A> {
    pub(super) fn new(runtime: Sim<A>, error: Option<Failure>) -> Self {
        Self {
            time: runtime.sim_time(),
            app: runtime,
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
            panic!("unwraped with errors: {error:?}");
        }
        self
    }
}

impl<A> RuntimeResult<A> {
    /// Returns an Err variant if some error occurred during simulation.
    ///
    /// # Errors
    ///
    /// Returns an error if any errors occurred during simulation.
    pub fn into_result(mut self) -> Result<RuntimeResult<A>, Failure> {
        match self.error.take() {
            None => Ok(self),
            Some(error) => Err(error),
        }
    }
}
