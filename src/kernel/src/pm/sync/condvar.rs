// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    clock,
    process::SleepError,
    ProcessManager,
};
use ::alloc::{
    collections::LinkedList,
    sync::Arc,
};
use ::core::{
    cell::RefCell,
    fmt,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Represents the inner state of a condition variable.
///
struct CondvarInner {
    sleeping: RefCell<LinkedList<(ProcessIdentifier, ThreadIdentifier)>>,
}

///
/// # Description
///
/// A type that represents a condition variable.
///
#[derive(Clone)]
pub struct Condvar {
    inner: Arc<CondvarInner>,
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
            inner: Arc::new(CondvarInner {
                sleeping: RefCell::new(LinkedList::new()),
            }),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the reference count of the target condition variable.
    ///
    /// # Returns
    ///
    /// The reference count of the target condition variable.
    ///
    /// # Safety
    ///
    /// This method by itself is safe, but using it correctly requires extra care. Another thread
    /// can change the strong count at any time, including potentially between calling this method
    /// and acting on the result.
    ///
    pub fn reference_count(&self) -> usize {
        Arc::strong_count(&self.inner)
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
        if let Some((pid, tid)) = self.inner.sleeping.borrow_mut().pop_front() {
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
        let idx: Option<usize> = self
            .inner
            .sleeping
            .borrow()
            .iter()
            .position(|&(p, _)| p == pid);

        // Remove process from sleeping queue.
        if let Some(at) = idx {
            let (pid, tid) = self.inner.sleeping.borrow_mut().remove(at);
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
    /// Upon successful completion, the number of threads that were awakened is returned. Otherwise,
    /// an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn notify_all(&self) -> Result<usize, Error> {
        let mut awakened: usize = 0;

        while let Some((pid, tid)) = self.inner.sleeping.borrow_mut().pop_front() {
            ProcessManager::wakeup(pid, tid)?;
            awakened += 1;
        }

        Ok(awakened)
    }

    ///
    /// # Description
    ///
    /// Waits on the condition variable.
    ///
    /// # Parameters
    ///
    /// - `alarm`: Optional alarm time.
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
    pub unsafe fn wait(&self, alarm: Option<SystemTime>) -> Result<(), SleepError> {
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

        // Check if alarm has already expired.
        if let Some(alarm) = alarm {
            let now: SystemTime = clock::now();
            if now >= alarm {
                error!(
                    "wait(): alarm has already expired (pid={:?}, tid={:?}, now={:?}, alarm={:?})",
                    pid, tid, now, alarm
                );
                return Err(SleepError::Generic(Error::new(
                    ErrorCode::OperationTimedOut,
                    "alarm has already expired",
                )));
            }
        }

        self.inner.sleeping.borrow_mut().push_back((pid, tid));

        ProcessManager::sleep(alarm)
    }
}

unsafe impl Send for CondvarInner {}

unsafe impl Sync for CondvarInner {}

impl fmt::Debug for Condvar {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "Condvar {{ sleeping: {:?} }}", self.inner.sleeping.borrow())
    }
}
