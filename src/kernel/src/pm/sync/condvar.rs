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
    collections::VecDeque,
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
    sleeping: RefCell<VecDeque<ThreadIdentifier>>,
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
                sleeping: RefCell::new(VecDeque::new()),
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

    /// Adds a synthetic waiter for an in-kernel notification-path test.
    #[cfg(feature = "test")]
    pub(crate) fn stage_test_waiter(&self, tid: ThreadIdentifier) {
        self.inner.sleeping.borrow_mut().push_back(tid);
    }

    /// Reports whether a synthetic waiter remains queued after a test notification.
    #[cfg(feature = "test")]
    pub(crate) fn has_test_waiter(&self, tid: ThreadIdentifier) -> bool {
        self.inner
            .sleeping
            .borrow()
            .iter()
            .any(|waiter| *waiter == tid)
    }

    ///
    /// # Description
    ///
    /// Wakes up a single thread that is waiting on a condition variable.
    ///
    /// # Return Value
    ///
    /// This function always returns `Ok` with the number of threads that were awakened (`1` if a
    /// genuinely waiting thread was woken, or `0` if the sleeping queue was empty or contained only
    /// stale waiters). Stale entries — whose thread already left the sleeping state (e.g., it timed
    /// out before this notification) — are skipped on a best-effort basis rather than reported as
    /// errors.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn notify_first(&self) -> Result<u32, Error> {
        // Wake the first thread that is still genuinely waiting. Stale entries — whose thread
        // already left the sleeping state (e.g., it timed out before this notification) — are
        // discarded as they are encountered, so the notification is delivered to a thread that is
        // actually waiting.
        while let Some(tid) = self.inner.sleeping.borrow_mut().pop_front() {
            if ProcessManager::wakeup_waiter(tid) {
                return Ok(1);
            }
        }

        Ok(0)
    }

    ///
    /// # Description
    ///
    /// Wakes up a specific thread that is waiting on the target condition variable.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the target thread.
    ///
    /// # Returns
    ///
    /// This function always returns `Ok(())`. If the target thread already left the sleeping state
    /// (e.g., it timed out before this notification), the wakeup is a best-effort no-op rather than
    /// an error.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn notify_thread(&self, tid: ThreadIdentifier) -> Result<(), Error> {
        // Find thread.
        let idx: Option<usize> = self.inner.sleeping.borrow().iter().position(|&t| t == tid);

        // Remove thread from sleeping queue and wake it up.
        if let Some(at) = idx {
            if let Some(notified_tid) = self.inner.sleeping.borrow_mut().remove(at) {
                debug_assert!(
                    notified_tid == tid,
                    "notify_thread(): tid does not match (expected: tid={tid:?}, got \
                     tid={notified_tid:?})",
                );
                // Best-effort: if the thread already left the sleeping state (e.g., it timed out),
                // there is nothing to wake and this is not an error.
                let _ = ProcessManager::wakeup_waiter(tid);
            }
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Wakes up all threads waiting on a condition variable.
    ///
    /// # Return Value
    ///
    /// This function always returns `Ok` with the number of threads that were awakened. Stale
    /// entries — whose thread already left the sleeping state (e.g., it timed out before this
    /// notification) — are skipped on a best-effort basis and not counted, rather than reported as
    /// errors.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn notify_all(&self) -> Result<u32, Error> {
        let mut awakened: u32 = 0; // Number of awakened threads.

        // Traverse the sleeping queue, waking up every thread that is still genuinely waiting.
        // Stale entries — whose thread already left the sleeping state (e.g., it timed out before
        // this notification) — are skipped rather than reported as errors.
        while let Some(tid) = self.inner.sleeping.borrow_mut().pop_front() {
            if ProcessManager::wakeup_waiter(tid) {
                awakened += 1;
            }
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
        let pid: ProcessIdentifier = unsafe { ProcessManager::get() }.get_pid();

        // Check if the kernel process is trying to sleep.
        if pid == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot sleep");
        }

        let tid: ThreadIdentifier = unsafe { ProcessManager::get() }.get_tid();

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

        self.inner.sleeping.borrow_mut().push_back(tid);

        match ProcessManager::sleep(alarm) {
            Ok(()) => Ok(()),
            Err(error) => {
                // Remove the thread from the sleeping queue if it was not woken up.
                self.inner.sleeping.borrow_mut().retain(|&t| t != tid);
                Err(error)
            },
        }
    }
}

unsafe impl Send for CondvarInner {}

unsafe impl Sync for CondvarInner {}

impl fmt::Debug for CondvarInner {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "CondvarInner {{ sleeping: {:?} }}", self.sleeping.borrow())
    }
}

impl fmt::Debug for Condvar {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "Condvar {{ sleeping: {:?} }}", self.inner.sleeping.borrow())
    }
}

impl Drop for CondvarInner {
    fn drop(&mut self) {
        if !self.sleeping.borrow().is_empty() {
            panic!("{self:?}");
        }
    }
}
