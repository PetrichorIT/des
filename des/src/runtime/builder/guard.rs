use crate::module::module_ctx_drop;
use std::sync::{Mutex, MutexGuard, TryLockError};

static GUARD: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub(crate) struct SimGuard {
    #[allow(unused)]
    guard: MutexGuard<'static, ()>,
}

impl SimGuard {
    pub(super) fn new() -> Self {
        let guard = GUARD.try_lock();
        let guard = match guard {
            Ok(guard) => guard,
            Err(e) => match e {
                TryLockError::WouldBlock => GUARD.lock().unwrap_or_else(|e| {
                    eprintln!("net-sim lock poisnoed: rebuilding lock");
                    e.into_inner()
                }),
                TryLockError::Poisoned(poisoned) => {
                    eprintln!("net-sim lock poisoned: rebuilding lock");
                    poisoned.into_inner()
                }
            },
        };

        Self { guard }
    }
}

impl Drop for SimGuard {
    fn drop(&mut self) {
        module_ctx_drop();
    }
}
