// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicBool,
    Ordering,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A spinlock.
///
pub struct Spinlock(AtomicBool);

///
/// # Description
///
/// A spinlock guard.
///
pub struct SpinlockGuard<'a>(&'a Spinlock);

//==================================================================================================
// Implementations
//==================================================================================================

impl Spinlock {
    ///
    /// # Description
    ///
    /// Instantiates an unlocked spinlock.
    ///
    /// # Returns
    ///
    /// A new unlocked spinlock.
    ///
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    ///
    /// # Description
    ///
    /// Locks the target spinlock. If the spinlock is already locked, the caller will spin until it
    /// the spinlock is unlocked and the lock is acquired.
    ///
    /// # Returns
    ///
    /// A spinlock guard that releases the lock when dropped.
    ///
    pub fn lock(&self) -> SpinlockGuard {
        loop {
            match self
                .0
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            {
                Ok(false) => break,
                _ => ::arch::cpu::pause(),
            }
        }

        SpinlockGuard(self)
    }
}

impl Drop for SpinlockGuard<'_> {
    fn drop(&mut self) {
        self.0 .0.store(false, Ordering::Release);
    }
}
