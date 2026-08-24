// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::SleepError,
    sync::condvar::Condvar,
};
use ::alloc::sync::Arc;
use ::core::{
    fmt,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sys::{
    error::Error,
    time::SystemTime,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents the inner data of a mutex.
///
pub struct MutexInner {
    /// Locked?
    locked: AtomicBool,
    /// Threads that are sleeping on the mutex.
    sleeping: Condvar,
}

///
/// # Description
///
/// A type that represents a mutex.
///
#[derive(Clone)]
pub struct Mutex(Arc<MutexInner>);

///
/// # Description
///
/// A type that represents a guard for a mutex.
///
pub struct MutexGuard {
    /// Reference to underlying mutex data.
    mutex: Arc<MutexInner>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl MutexInner {
    ///
    /// # Description
    ///
    /// Releases the mutex.
    ///
    /// # Returns
    ///
    /// Upon success, empty result is returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function leads to undefined behavior if any of the following conditions are violated:
    /// - The lock is held by the caller.
    ///
    unsafe fn unlock_unchecked(&self) -> Result<(), Error> {
        self.locked.store(false, Ordering::Relaxed);
        self.sleeping.notify_first().map(|_awakened| ())
    }
}

impl Mutex {
    ///
    /// # Description
    ///
    /// Initializes a new unlocked mutex.
    ///
    /// # Parameters
    ///
    /// - `value`: Initial value of the mutex.
    ///
    /// # Returns
    ///
    /// A new mutex.
    ///
    pub fn new() -> Self {
        Self(Arc::new(MutexInner {
            locked: AtomicBool::new(false),
            sleeping: Condvar::new(),
        }))
    }

    ///
    /// # Description
    ///
    /// Returns the reference count of the mutex.
    ///
    /// # Returns
    ///
    /// The reference count of the mutex.
    ///
    /// # Safety
    ///
    /// This method by itself is safe, but using it correctly requires extra care. Another thread
    /// can change the strong count at any time, including potentially between calling this method
    /// and acting on the result.
    ///
    pub fn reference_count(&self) -> usize {
        Arc::strong_count(&self.0)
    }

    ///
    /// # Description
    ///
    /// Attempts to lock the target mutex.
    ///
    /// # Returns
    ///
    /// Upon success, a guard is returned. Upon failure, an error is returned instead.
    ///
    pub fn try_lock(&self) -> Result<MutexGuard, ()> {
        if self
            .0
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            Err(())
        } else {
            Ok(MutexGuard {
                mutex: self.0.clone(),
            })
        }
    }

    ///
    /// # Description
    ///
    /// Acquires the mutex.
    ///
    /// # Parameters
    ///
    /// - `timeout`: Timeout for the mutex.
    ///
    /// # Returns
    ///
    /// Upon success, empty result is returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function panics if the kernel process tries to sleep.
    ///
    /// This function is unsafe because it blocks the calling thread until it is woken up by another
    /// thread.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    /// - This function is invoked without holding any resources.
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn lock(&self, timeout: Option<SystemTime>) -> Result<MutexGuard, SleepError> {
        loop {
            // Attempt to acquire the mutex.
            match self.try_lock() {
                // Success.
                Ok(guard) => return Ok(guard),
                // Failed to acquire the mutex.
                Err(()) => {
                    self.0.sleeping.wait(timeout)?;
                },
            }
        }
    }
}

impl fmt::Debug for MutexGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MutexGuard {{ locked: {:?}, condvar: {:?} }}",
            self.mutex.locked.load(Ordering::Relaxed),
            self.mutex.sleeping
        )
    }
}

impl Drop for MutexGuard {
    fn drop(&mut self) {
        // Safety: The lock is ensured to be held by the caller.
        if let Err(error) = unsafe { self.mutex.unlock_unchecked() } {
            warn!("failed to unlock mutex (self={self:?}, error={error:?})");
        }
    }
}
