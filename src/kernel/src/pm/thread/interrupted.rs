// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    sync::condvar::Condvar,
    thread::{
        state::ThreadState,
        ReadyThread,
    },
};
use ::alloc::boxed::Box;
use ::core::fmt::Debug;
use ::sys::pm::ThreadIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum InterruptReason {
    /// Process was killed.
    Killed,
    /// Timer expired.
    TimedOut,
    /// A deliverable, caught signal interrupted the call.
    Signaled,
}

///
/// # Description
///
/// This structure represents a thread that has been interrupted.
///
#[derive(Debug)]
pub struct InterruptedThread {
    state: Box<ThreadState>,
    reason: InterruptReason,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl InterruptedThread {
    ///
    /// # Description
    ///
    /// Creates a new interrupted thread.
    ///
    /// # Parameters
    ///
    /// - `state`: The thread state.
    /// - `reason`: The reason for the interruption.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of an [`InterruptedThread`].
    ///
    pub(super) fn from_state(state: Box<ThreadState>, reason: InterruptReason) -> Self {
        Self { state, reason }
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the interrupted thread.
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
    /// Resumes the interrupted thread by transitioning it to ready state.
    ///
    /// # Returns
    ///
    /// This function returns a [`ReadyThread`] instance.
    ///
    pub fn resume(mut self) -> ReadyThread {
        self.state.set_interrupt_reason(self.reason);
        ReadyThread::from_state(self.state)
    }

    ///
    /// # Description
    ///
    /// Overrides the interrupt reason so that the thread terminates when it is next resumed.
    ///
    /// This is used to convert an already-interrupted thread (e.g. one that timed out) into a
    /// killed thread when its process receives a fatal signal.
    ///
    pub fn set_killed(&mut self) {
        self.reason = InterruptReason::Killed;
    }

    ///
    /// # Description
    ///
    /// Returns the reason why the thread was interrupted.
    ///
    /// # Returns
    ///
    /// A reference to the [`InterruptReason`] of the interrupted thread.
    ///
    #[cfg(feature = "test")]
    pub fn reason(&self) -> &InterruptReason {
        &self.reason
    }

    ///
    /// # Description
    ///
    /// Returns the join condition variable of the interrupted thread.
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
    /// Returns whether the interrupted thread is detached.
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
    /// Marks the interrupted thread as detached.
    ///
    pub fn set_detached(&mut self) {
        self.state.set_detached();
    }
}
