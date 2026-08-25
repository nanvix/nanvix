// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================
use crate::{
    hal::arch::{
        x86::cpu::FpuState,
        ContextInformation,
    },
    mm::{
        kstack::KernelStack,
        ustack::UserStack,
    },
    pm::thread::state::ThreadState,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::VirtualAddress,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Modules
//==================================================================================================

mod interrupted;
mod ready;
mod running;
mod sleeping;
mod state;
mod zombie;

//==================================================================================================
// Exports
//==================================================================================================

pub use interrupted::{
    InterruptReason,
    InterruptedThread,
};
pub use ready::ReadyThread;
pub use running::RunningThread;
pub use sleeping::SleepingThread;
pub use state::KcallRestart;
pub use zombie::ZombieThread;
pub(crate) use zombie::{
    PendingThreadTermination,
    ZombieThreadTransition,
};

//==================================================================================================
// Thread Reference
//==================================================================================================

///
/// # Description
///
/// A reference to a thread.
///
pub enum ThreadRef<'a> {
    /// A reference to a ready thread.
    Ready(&'a ReadyThread),
    /// A reference to a running thread.
    Running(&'a RunningThread),
    /// A reference to a sleeping thread.
    Sleeping(&'a SleepingThread),
    /// A reference to an interrupted thread.
    Interrupted(&'a InterruptedThread),
    /// A reference to a zombie thread.
    Zombie(&'a ZombieThread),
}

impl<'a> ThreadRef<'a> {
    ///
    /// # Description
    ///
    /// Returns a reference to the thread's state.
    ///
    /// # Return Value
    ///
    /// This function returns a reference to the thread's state.
    ///
    #[allow(dead_code)]
    pub fn thread_state(&self) -> &ThreadState {
        match self {
            ThreadRef::Ready(thread) => thread.thread_state(),
            ThreadRef::Running(thread) => thread.thread_state(),
            ThreadRef::Sleeping(thread) => thread.thread_state(),
            ThreadRef::Interrupted(thread) => thread.thread_state(),
            ThreadRef::Zombie(thread) => thread.thread_state(),
        }
    }

    ///
    /// # Description
    ///
    /// Returns whether the referenced thread is detached.
    ///
    /// # Return Value
    ///
    /// This function returns `true` if the thread is detached, `false` otherwise.
    ///
    #[cfg(feature = "test")]
    pub fn is_detached(&self) -> bool {
        match self {
            ThreadRef::Ready(thread) => thread.is_detached(),
            ThreadRef::Running(thread) => thread.is_detached(),
            ThreadRef::Sleeping(thread) => thread.is_detached(),
            ThreadRef::Interrupted(thread) => thread.is_detached(),
            ThreadRef::Zombie(thread) => thread.is_detached(),
        }
    }
}

//==================================================================================================
// Mutable Thread Reference
//==================================================================================================

///
/// # Description
///
/// A mutable reference to a thread.
///
pub enum ThreadRefMut<'a> {
    /// A mutable reference to a ready thread.
    Ready(&'a mut ReadyThread),
    /// A mutable reference to a running thread.
    Running(&'a mut RunningThread),
    /// A mutable reference to a sleeping thread.
    Sleeping(&'a mut SleepingThread),
    /// A mutable reference to an interrupted thread.
    Interrupted(&'a mut InterruptedThread),
    /// A mutable reference to a zombie thread.
    Zombie(&'a mut ZombieThread),
}

impl<'a> ThreadRefMut<'a> {
    ///
    /// # Description
    ///
    /// Returns a mutable reference to the thread's state.
    ///
    /// # Return Value
    ///
    /// This function returns a mutable reference to the thread's state.
    ///
    pub fn thread_state_mut(&mut self) -> &mut ThreadState {
        match self {
            ThreadRefMut::Ready(thread) => thread.thread_state_mut(),
            ThreadRefMut::Running(thread) => thread.thread_state_mut(),
            ThreadRefMut::Sleeping(thread) => thread.thread_state_mut(),
            ThreadRefMut::Interrupted(thread) => thread.thread_state_mut(),
            ThreadRefMut::Zombie(thread) => thread.thread_state_mut(),
        }
    }
}

//==================================================================================================
// Thread Manager
//==================================================================================================

///
/// # Description
///
/// This structure represents a thread manager that is responsible for creating and managing threads.
///
pub struct ThreadManager {
    /// Next thread identifier to be assigned.
    next_id: ThreadIdentifier,
    /// Number of threads that have been created but not yet reaped (joined or harvested).
    /// Initialized to 1 to account for the kernel thread created in [`ThreadManager::new()`].
    live_count: usize,
}

impl ThreadManager {
    ///
    /// # Description
    ///
    /// Creates a new thread manager and initializes the kernel thread.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the kernel thread and a new instance of a [`ThreadManager`].
    ///
    fn new() -> (ReadyThread, Self) {
        let kernel: ReadyThread = {
            ReadyThread::new(
                From::<i32>::from(0),
                None,
                None,
                None,
                None,
                ContextInformation::default(),
                // SAFETY: calls to FpuState::new are synchronized.
                unsafe { FpuState::new() },
            )
        };
        (
            kernel,
            Self {
                next_id: From::<i32>::from(1),
                live_count: 1,
            },
        )
    }

    ///
    /// # Description
    ///
    /// Reserves the next thread identifier, performing a checked increment.
    ///
    /// # Returns
    ///
    /// Upon success, this function returns a tuple containing the reserved [`ThreadIdentifier`]
    /// and the next [`ThreadIdentifier`] value. The caller must commit the next identifier via
    /// [`commit_next_tid`] after all fallible operations have succeeded.
    ///
    /// # Errors
    ///
    /// This function returns an error if the thread identifier would overflow.
    ///
    /// # Known Bugs
    ///
    /// - FIXME (#1440): thread identifiers are never recycled, so a fork bomb or repeated
    ///   `create_thread` calls can exhaust the identifier space.
    ///
    pub(crate) fn try_next_tid(&self) -> Result<(ThreadIdentifier, ThreadIdentifier), Error> {
        // Reject if the system-wide thread cap would be exceeded.
        if self.live_count >= ::config::kernel::MAX_THREADS {
            let reason: &str = "system-wide thread limit reached";
            error!(
                "{reason} (live_count={}, max_threads={})",
                self.live_count,
                ::config::kernel::MAX_THREADS
            );
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        let id: ThreadIdentifier = self.next_id;
        let raw_id: i32 = <i32>::from(self.next_id);
        let next_raw_id: i32 = match raw_id.checked_add(1) {
            Some(val) => val,
            None => {
                let reason: &str = "thread identifier overflow";
                error!("{reason} (next_id={raw_id:?})");
                return Err(Error::new(ErrorCode::ValueOverflow, reason));
            },
        };
        Ok((id, ThreadIdentifier::from(next_raw_id)))
    }

    ///
    /// # Description
    ///
    /// Commits the next thread identifier after all fallible operations have succeeded.
    ///
    /// # Parameters
    ///
    /// - `next_tid`: The next thread identifier (obtained from [`try_next_tid`]).
    ///
    pub(crate) fn commit_next_tid(&mut self, next_tid: ThreadIdentifier) {
        self.next_id = next_tid;
        self.live_count += 1;
    }

    ///
    /// # Description
    ///
    /// Notifies the thread manager that a thread has been reaped (joined or harvested as a zombie).
    /// This decrements the live thread count, freeing a slot for future thread creation.
    ///
    pub(crate) fn on_thread_reaped(&mut self) {
        // Use a runtime assert (not debug_assert) so that an accounting bug in release builds
        // panics instead of silently wrapping usize and permanently breaking admission control.
        // The kernel thread is never reaped, so live_count must be at least 2 here (the kernel
        // thread plus the thread being reaped).
        assert!(
            self.live_count > 1,
            "live_count underflow: kernel thread must always remain counted"
        );
        self.live_count -= 1;
    }

    ///
    /// # Description
    ///
    /// Creates a new thread with the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `id`: Pre-allocated thread identifier (obtained from [`try_next_tid`]).
    /// - `kernel_stack`: Optional kernel stack for the thread.
    /// - `user_stack`: Optional user stack for the thread.
    /// - `user_tda`: Optional base address to user-space thread data area.
    /// - `context`: Execution context for the thread.
    ///
    /// # Returns
    ///
    /// This function returns a new [`ReadyThread`] instance.
    ///
    pub fn create_thread(
        &self,
        id: ThreadIdentifier,
        kernel_stack: Option<KernelStack>,
        user_stack: Option<UserStack>,
        user_tda: Option<VirtualAddress>,
        context: ContextInformation,
    ) -> ReadyThread {
        ReadyThread::new(
            id,
            None,
            kernel_stack,
            user_stack,
            user_tda,
            context,
            // SAFETY: calls to FpuState::new are synchronized.
            unsafe { FpuState::new() },
        )
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the thread manager.
///
/// # Returns
///
/// This function returns a tuple containing the kernel thread and a new instance of a [`ThreadManager`].
///
pub fn init() -> (ReadyThread, ThreadManager) {
    // TODO: check for double initialization.

    ThreadManager::new()
}
