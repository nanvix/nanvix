// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::condvar::Condvar,
    thread::{
        interrupted::InterruptReason,
        state::ThreadState,
        InterruptedThread,
        ReadyThread,
    },
};
use ::alloc::boxed::Box;
use ::core::fmt::Debug;
use ::sys::{
    mm::VirtualAddress,
    pm::ThreadIdentifier,
    time::SystemTime,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents a thread that is sleeping.
///
#[derive(Debug)]
pub struct SleepingThread {
    /// Thread state.
    state: Box<ThreadState>,
    /// Optional alarm time for waking up the thread.
    alarm: Option<SystemTime>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SleepingThread {
    ///
    /// # Description
    ///
    /// Creates a sleeping thread from an existing thread state and alarm time.
    ///
    /// # Parameters
    ///
    /// - `state`: The thread state.
    /// - `alarm`: Optional alarm time for waking up the thread.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a [`SleepingThread`].
    ///
    pub(super) fn from_state(state: Box<ThreadState>, alarm: Option<SystemTime>) -> Self {
        Self { state, alarm }
    }

    ///
    /// # Description
    ///
    /// Wakes up the sleeping thread and transitions it to ready state.
    ///
    /// # Returns
    ///
    /// This function returns a [`ReadyThread`] instance.
    ///
    pub fn wakeup(self) -> ReadyThread {
        ReadyThread::from_state(self.state)
    }

    ///
    /// # Description
    ///
    /// Interrupts the sleeping thread with the specified reason.
    ///
    /// # Parameters
    ///
    /// - `reason`: The reason for the interruption.
    ///
    /// # Returns
    ///
    /// This function returns an [`InterruptedThread`] instance.
    ///
    pub fn interrupt(self, reason: InterruptReason) -> InterruptedThread {
        InterruptedThread::from_state(self.state, reason)
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the sleeping thread.
    ///
    /// # Returns
    ///
    /// This function returns the thread identifier.
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
    /// Returns the join condition variable of the sleeping thread.
    ///
    /// # Returns
    ///
    /// This function returns the join condition variable.
    ///
    pub fn join_cond(&self) -> Condvar {
        self.state.join_cond()
    }

    ///
    /// # Description
    ///
    /// Returns whether the sleeping thread is detached.
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
    /// Marks the sleeping thread as detached.
    ///
    pub fn set_detached(&mut self) {
        self.state.set_detached();
    }

    ///
    /// # Description
    ///
    /// Returns the alarm time of the sleeping thread.
    ///
    /// # Returns
    ///
    /// This function returns the optional alarm time for waking up the thread.
    ///
    pub fn alarm(&self) -> Option<SystemTime> {
        self.alarm
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
