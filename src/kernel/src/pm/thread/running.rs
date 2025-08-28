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
        },
    },
};
use ::alloc::boxed::Box;
use ::core::fmt::Debug;
use ::sys::{
    pm::{
        MutexAddress,
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
    /// This function returns a tuple containing the sleeping thread and a mutable pointer to the execution context.
    ///
    pub fn sleep(
        mut self,
        alarm: Option<SystemTime>,
    ) -> (SleepingThread, *mut ContextInformation, *mut FpuState) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        let fpu_state: *mut FpuState = self.state.fpu_state_mut();
        (SleepingThread::from_state(self.state, alarm), ctx, fpu_state)
    }

    ///
    /// # Description
    ///
    /// Schedules the running thread by transitioning it to ready state.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the ready thread and a mutable pointer to the execution context.
    ///
    pub fn schedule(mut self) -> (ReadyThread, *mut ContextInformation, *mut FpuState) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        let fpu_state: *mut FpuState = self.state.fpu_state_mut();
        (ReadyThread::from_state(self.state), ctx, fpu_state)
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
    /// Terminates the running thread with the specified exit status.
    ///
    /// # Parameters
    ///
    /// - `status`: The exit status of the thread.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the zombie thread and a mutable pointer to the execution context.
    ///
    pub fn exit(
        mut self,
        status: ExitStatus,
    ) -> (ZombieThread, *mut ContextInformation, *mut FpuState) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        let fpu_state: *mut FpuState = self.state.fpu_state_mut();
        (ZombieThread::from_state(self.state, status), ctx, fpu_state)
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
}
