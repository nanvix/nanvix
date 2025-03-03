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
    pub fn notify_first(&self) -> Result<(), Error> {
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
    pub fn notify_process(&self, pid: ProcessIdentifier) -> Result<(), Error> {
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
    /// Waits on the condition variable.
    ///
    /// # Safety
    ///
    /// This function panics if the kernel process tries to sleep.
    ///
    pub fn wait(&self) -> Result<(), SleepError> {
        let pid: ProcessIdentifier = ProcessManager::get_pid().map_err(SleepError::Generic)?;

        // Check if the kernel process is trying to sleep.
        if pid == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot sleep");
        }

        let tid: ThreadIdentifier = ProcessManager::get_tid().map_err(SleepError::Generic)?;

        self.sleeping.borrow_mut().push_back((pid, tid));

        // SAFETY: the calling process is not the kernel.
        unsafe { ProcessManager::sleep() }
    }
}

unsafe impl Send for Condvar {}

unsafe impl Sync for Condvar {}
