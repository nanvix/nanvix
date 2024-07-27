// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::Error,
    pm::sync::condvar::Condvar,
};
use ::core::cell::RefCell;
use ::kcall::ErrorCode;

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
    value: RefCell<usize>,
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
    ///
    pub fn new(value: usize) -> Self {
        Self {
            value: RefCell::new(value),
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
    pub fn down(&self) -> Result<(), Error> {
        while *self.value.borrow() == 0 {
            self.sleeping.wait()?;
        }

        *self.value.borrow_mut() -= 1;

        Ok(())
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
        if *self.value.borrow() == 0 {
            return Err(Error::new(ErrorCode::OperationWouldBlock, "semaphore is busy"));
        }

        *self.value.borrow_mut() -= 1;

        Ok(())
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
    pub fn up(&self) -> Result<(), Error> {
        *self.value.borrow_mut() += 1;
        self.sleeping.notify()
    }
}

unsafe impl Send for Semaphore {}

unsafe impl Sync for Semaphore {}
