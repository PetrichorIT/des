use crate::net::{buf_drop, buf_init, module::module_ctx_drop, Globals};
use std::sync::{Mutex, MutexGuard, TryLockError, Weak};

static GUARD: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub(super) struct SimStaticsGuard {
    #[allow(unused)]
    guard: MutexGuard<'static, ()>,
}

impl SimStaticsGuard {
    pub(super) fn new(globals: Weak<Globals>) -> Self {
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

        buf_init(globals);
        Self { guard }
    }
}

impl Drop for SimStaticsGuard {
    fn drop(&mut self) {
        buf_drop();
        module_ctx_drop();
    }
}
