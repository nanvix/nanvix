// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::SleepError,
    ProcessManager,
};
use ::alloc::collections::LinkedList;
use ::core::cell::RefCell;
use ::sys::{
    error::Error,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a condition variable.
///
pub struct Condvar {
    /// Threads that are sleeping on the condition variable.
    sleeping: RefCell<LinkedList<(ProcessIdentifier, ThreadIdentifier)>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Condvar {
    ///
    /// # Description
    ///
    /// Initializes a new condition variable.
    ///
    /// # Returns
    ///
    /// A new condition variable.
    ///
    pub fn new() -> Self {
        Self {
            sleeping: RefCell::new(LinkedList::new()),
        }
    }

    ///
    /// # Description
    ///
    /// Wakes a single thread that is waiting on the target condition variable.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn notify_first(&self) -> Result<(), Error> {
        if let Some((pid, tid)) = self.sleeping.borrow_mut().pop_front() {
            ProcessManager::wakeup(pid, tid)?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Wakes up all threads of a process that are waiting on the target condition variable.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the target process.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn notify_process(&self, pid: ProcessIdentifier) -> Result<(), Error> {
        // Find process.
        let idx: Option<usize> = self.sleeping.borrow().iter().position(|&(p, _)| p == pid);

        // Remove process from sleeping queue.
        if let Some(at) = idx {
            let (pid, tid) = self.sleeping.borrow_mut().remove(at);
            ProcessManager::wakeup(pid, tid)?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Wakes up all threads waiting on the target condition variable.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn notify_all(&self) -> Result<(), Error> {
        while let Some((pid, tid)) = self.sleeping.borrow_mut().pop_front() {
            ProcessManager::wakeup(pid, tid)?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Waits on the condition variable.
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
    pub unsafe fn wait(&self) -> Result<(), SleepError> {
        let pid: ProcessIdentifier = unsafe { ProcessManager::get() }
            .get_pid()
            .map_err(SleepError::Generic)?;

        // Check if the kernel process is trying to sleep.
        if pid == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot sleep");
        }

        let tid: ThreadIdentifier = unsafe { ProcessManager::get() }
            .get_tid()
            .map_err(SleepError::Generic)?;

        self.sleeping.borrow_mut().push_back((pid, tid));

        ProcessManager::sleep()
    }
}

unsafe impl Send for Condvar {}

unsafe impl Sync for Condvar {}
