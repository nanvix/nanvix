// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    pm::{
        sync::{
            condvar::Condvar,
            mutex::MutexGuard,
        },
        thread::{
            state::ThreadState,
            ReadyThread,
            SleepingThread,
            ZombieThread,
            ZombieThreadTransition,
        },
    },
};
use ::alloc::boxed::Box;
use ::core::fmt::Debug;
use ::sys::{
    error::Error,
    mm::VirtualAddress,
    pm::{
        MutexAddress,
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
    ExitStatus,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents a thread that is currently running.
///
#[derive(Debug)]
pub struct RunningThread {
    /// Thread state.
    state: Box<ThreadState>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RunningThread {
    ///
    /// # Description
    ///
    /// Creates a running thread from an existing thread state.
    ///
    /// # Parameters
    ///
    /// - `state`: The thread state.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a [`RunningThread`].
    ///
    pub(super) fn from_state(state: Box<ThreadState>) -> Self {
        Self { state }
    }

    ///
    /// # Description
    ///
    /// Transitions the running thread to sleeping state.
    ///
    /// # Parameters
    ///
    /// - `alarm`: Optional alarm time for the sleeping thread.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the sleeping thread and a mutable pointer to the
    /// execution context.
    ///
    pub fn sleep(mut self, alarm: Option<SystemTime>) -> (SleepingThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        (SleepingThread::from_state(self.state, alarm), ctx)
    }

    ///
    /// # Description
    ///
    /// Schedules the running thread by transitioning it to ready state.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the ready thread and a mutable pointer to the
    /// execution context.
    ///
    pub fn schedule(mut self) -> (ReadyThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        (ReadyThread::from_state(self.state), ctx)
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the target thread.
    ///
    /// # Returns
    ///
    /// The identifier of the target thread.
    ///
    pub fn id(&self) -> ThreadIdentifier {
        self.state.id()
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the thread state.
    ///
    /// # Returns
    ///
    /// This function returns a reference to the thread state.
    ///
    pub fn thread_state(&self) -> &ThreadState {
        &self.state
    }

    ///
    /// # Description
    ///
    /// Returns a mutable reference to the thread state.
    ///
    /// # Returns
    ///
    /// This function returns a mutable reference to the thread state.
    ///
    pub fn thread_state_mut(&mut self) -> &mut ThreadState {
        &mut self.state
    }

    ///
    /// # Description
    ///
    /// Wakes up all threads waiting on the join condition variable.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn join_cond(&self) -> Condvar {
        // NOTE: we must wake up all, otherwise some threads can be left waiting forever.
        self.state.join_cond()
    }

    ///
    /// # Description
    ///
    /// Returns whether the running thread is detached.
    ///
    /// # Returns
    ///
    /// This function returns `true` if the thread is detached, `false` otherwise.
    ///
    pub fn is_detached(&self) -> bool {
        self.state.is_detached()
    }

    ///
    /// # Description
    ///
    /// Marks the running thread as detached.
    ///
    pub fn set_detached(&mut self) {
        self.state.set_detached();
    }

    ///
    /// # Description
    ///
    /// Terminates the running thread with the specified exit status.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process that owns this thread.
    /// - `status`: The exit status of the thread.
    ///
    /// # Returns
    ///
    /// This function returns the must-use zombie-thread transition and a mutable pointer to the
    /// execution context.
    ///
    /// # Panics
    ///
    /// This function panics if the thread does not own a termination credit.
    ///
    pub fn exit(
        mut self,
        pid: ProcessIdentifier,
        status: ExitStatus,
    ) -> (ZombieThreadTransition, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        (ZombieThread::from_state(pid, self.state, status), ctx)
    }

    ///
    /// # Description
    ///
    /// Retires the running thread as part of an `execv()`, returning a transition whose zombie owns
    /// the outgoing thread's kernel stack and execution context and whose pending record owns the
    /// termination reservation.
    ///
    /// Unlike [`Self::exit`], this drops the thread's user-stack handle before zombifying it. The
    /// user stack lives in the outgoing address space, which `execv()` reclaims wholesale; keeping
    /// the handle would cause the later zombie harvest to unmap the stack's virtual range from the
    /// *new* address space, since every image places its stack at the same fixed virtual address.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process that owns this thread.
    ///
    /// # Returns
    ///
    /// A tuple with the must-use zombie-thread transition and a pointer to its execution context.
    /// The context remains valid until the zombie is harvested, which the caller defers until after
    /// the context switch into the new image.
    ///
    /// # Panics
    ///
    /// This function panics if the thread does not own a termination credit.
    ///
    pub fn exit_for_exec(
        mut self,
        pid: ProcessIdentifier,
    ) -> (ZombieThreadTransition, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        // Drop the user-stack handle (a frame-less address holder) so harvest will not attempt to
        // unmap its range from the new address space.
        let _ = self.state.take_user_stack();
        (ZombieThread::from_state(pid, self.state, ExitStatus::ok()), ctx)
    }

    ///
    /// # Description
    ///
    /// Stores a mutex guard in the target thread.
    ///
    /// # Parameters
    ///
    /// - `mutex_addr`: Address of the mutex.
    /// - `guard`: Mutex guard.
    ///
    pub fn put_mutex_guard(&mut self, mutex_addr: MutexAddress, guard: MutexGuard) {
        self.state.store_mutex_guard(mutex_addr, guard);
    }

    ///
    /// # Description
    ///
    /// Returns the mutex guard associated with the target thread.
    ///
    /// # Parameters
    ///
    /// - `mutex_addr`: Address of the mutex.
    ///
    /// # Returns
    ///
    /// If the mutex guard is found, it is returned. Otherwise, `None` is returned instead.
    ///
    pub fn take_mutex_guard(&mut self, mutex_addr: MutexAddress) -> Option<MutexGuard> {
        self.state.take_mutex_guard(mutex_addr)
    }

    ///
    /// # Description
    ///
    /// Checks the guard watermark of the kernel stack for corruption.
    ///
    /// # Returns
    ///
    /// Upon success (watermark intact or no kernel stack), `Ok(())` is returned. Upon failure
    /// (watermark corrupted), an error is returned.
    ///
    #[inline]
    pub fn check_guard_watermark(&self) -> Result<(), Error> {
        self.state.check_guard_watermark()
    }

    ///
    /// # Description
    ///
    /// Returns the guard threshold of the running thread's kernel stack, if any.
    ///
    /// # Returns
    ///
    /// The guard threshold value, or `None` if the thread has no kernel stack.
    ///
    #[cfg(feature = "exception-stack-guard")]
    #[inline]
    pub fn guard_threshold(&self) -> Option<u32> {
        self.state.guard_threshold()
    }

    ///
    /// # Description
    ///
    /// Sets the base address for the user-space thread data area for the target thread.
    ///
    /// # Parameters
    ///
    /// - `user_tda`: Optional thread data area pointer to set.
    ///
    pub fn set_thread_data_area(&mut self, user_tda: Option<VirtualAddress>) {
        self.state.store_thread_data_area(user_tda);
    }

    ///
    /// # Description
    ///
    /// Gets the base address for user-space thread data area for the target thread.
    ///
    /// # Returns
    ///
    /// This function returns the optional base address for user-space thread data area for the
    /// target thread.
    ///
    pub fn get_thread_data_area(&self) -> Option<VirtualAddress> {
        self.state.get_thread_data_area()
    }
}
