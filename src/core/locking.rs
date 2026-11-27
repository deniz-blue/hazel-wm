use std::sync::{Mutex, MutexGuard};

use tracing::warn;

pub trait LockRecovering<T> {
    fn lock_recovering(&self) -> MutexGuard<'_, T>;
}

impl<T> LockRecovering<T> for Mutex<T> {
    fn lock_recovering(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| {
            warn!("a mutex was poisoned by an earlier panic, using its data anyway");
            poisoned.into_inner()
        })
    }
}
