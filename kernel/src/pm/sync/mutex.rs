// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Error,
    pm::sync::condvar::Condvar,
};
use alloc::sync::Arc;
use core::cell::RefCell;

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
    locked: RefCell<bool>,
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
        *self.locked.borrow_mut() = false;
        self.sleeping.notify()
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
            locked: RefCell::new(false),
            sleeping: Condvar::new(),
        }))
    }

    ///
    /// # Description
    ///
    /// Acquires the mutex.
    ///
    /// # Returns
    ///
    /// Upon success, empty result is returned. Upon failure, an error is returned instead.
    ///
    pub fn lock(&self) -> Result<MutexGuard, Error> {
        while *self.0.locked.borrow() {
            self.0.sleeping.wait()?;
        }

        *self.0.locked.borrow_mut() = true;

        Ok(MutexGuard {
            mutex: self.0.clone(),
        })
    }
}

impl Drop for MutexGuard {
    fn drop(&mut self) {
        // Safety: The lock is ensured to be held by the caller.
        if let Err(e) = unsafe { self.mutex.unlock_unchecked() } {
            warn!("failed to unlock mutex: {:?}", e);
        }
    }
}

unsafe impl Send for MutexInner {}

unsafe impl Sync for MutexInner {}
