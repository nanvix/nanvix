// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::condvar::Condvar,
    SleepError,
};
use ::core::sync::atomic::{
    AtomicUsize,
    Ordering,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a semaphore.
///
pub struct Semaphore {
    /// Value.
    value: AtomicUsize,
    /// Threads that are sleeping on the semaphore.
    sleeping: Condvar,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Semaphore {
    ///
    /// # Description
    ///
    /// Initializes a new semaphore.
    ///
    /// # Parameters
    ///
    /// - `value`: Initial value of the semaphore.
    ///
    /// # Returns
    ///
    /// A new semaphore.
    pub fn new(value: usize) -> Self {
        Self {
            value: AtomicUsize::new(value),
            sleeping: Condvar::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Acquires the semaphore.
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
    ///
    pub unsafe fn down(&self) -> Result<(), SleepError> {
        loop {
            if self
                .value
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                    if value == 0 {
                        None
                    } else {
                        Some(value - 1)
                    }
                })
                .is_ok()
            {
                return Ok(());
            }

            self.sleeping.wait(None)?;
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to acquire the semaphore.
    ///
    /// # Returns
    ///
    /// If the semaphore is not busy, it is acquired and an empty result is returned. If the
    /// semaphore is busy, [`ErrorCode::OperationWouldBlock`] is returned instead. If an error
    /// occurs, an error is returned instead.
    ///
    pub fn try_down(&self) -> Result<(), Error> {
        if self
            .value
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                if value == 0 {
                    None
                } else {
                    Some(value - 1)
                }
            })
            .is_ok()
        {
            return Ok(());
        }

        Err(Error::new(ErrorCode::TryAgain, "semaphore is busy"))
    }

    ///
    /// # Description
    ///
    /// Releases the semaphore.
    ///
    /// # Returns
    ///
    /// Upon success, empty result is returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because:
    /// - It mutates global variables without explicit synchronization.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The caller is runner with interrupts disabled.
    /// - The calling process does not hold a reference to the process manager.
    ///
    /// # Notes
    ///
    /// - This function does not trigger in an immediate context switch.
    ///
    pub unsafe fn up(&self) -> Result<(), Error> {
        self.value.fetch_add(1, Ordering::SeqCst);

        if let Err(error) = self.sleeping.notify_first().map(|_awakened| ()) {
            self.value.fetch_sub(1, Ordering::SeqCst);
            return Err(error);
        }

        Ok(())
    }
}
