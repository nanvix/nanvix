// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mm::{
        kstack::KernelStack,
        ustack::UserStack,
    },
    pm::{
        process::ThreadLifecycleTerminationCredit,
        thread::state::ThreadState,
    },
};
use ::alloc::boxed::Box;
use ::core::fmt::Debug;
use ::sys::{
    event::ThreadTerminationInfo,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    ExitStatus,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents a thread that has terminated and is waiting to be harvested.
///
#[derive(Debug)]
pub struct ZombieThread {
    /// Exit status of the terminated thread.
    status: ExitStatus,
    /// Thread state.
    state: Box<ThreadState>,
}

/// Thread-termination record and the capacity credit transferred out of a live thread.
#[must_use]
pub(crate) struct PendingThreadTermination {
    info: ThreadTerminationInfo,
    credit: ThreadLifecycleTerminationCredit,
}

impl PendingThreadTermination {
    ///
    /// # Description
    ///
    /// Splits this pending termination into its record and reserved capacity credit.
    ///
    /// # Returns
    ///
    /// A tuple containing the thread-termination record and its reserved capacity credit.
    ///
    pub(crate) fn into_parts(self) -> (ThreadTerminationInfo, ThreadLifecycleTerminationCredit) {
        (self.info, self.credit)
    }
}

/// Result of the authoritative live-to-zombie thread transition.
#[must_use]
pub(crate) struct ZombieThreadTransition {
    zombie: ZombieThread,
    pending: PendingThreadTermination,
}

impl ZombieThreadTransition {
    ///
    /// # Description
    ///
    /// Splits this transition into the zombie and its pending termination record.
    ///
    /// # Returns
    ///
    /// A tuple containing the zombie thread and its pending termination record.
    ///
    pub(crate) fn into_parts(self) -> (ZombieThread, PendingThreadTermination) {
        (self.zombie, self.pending)
    }
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ZombieThread {
    ///
    /// # Description
    ///
    /// Creates a zombie thread from an existing thread state and exit status.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process that owns the thread.
    /// - `state`: The thread state.
    /// - `status`: The exit status of the terminated thread.
    ///
    /// # Returns
    ///
    /// This function returns the must-use zombie-thread transition.
    ///
    /// # Panics
    ///
    /// This function panics if `state` does not own a thread termination credit.
    ///
    pub(super) fn from_state(
        pid: ProcessIdentifier,
        mut state: Box<ThreadState>,
        status: ExitStatus,
    ) -> ZombieThreadTransition {
        let info: ThreadTerminationInfo = ThreadTerminationInfo::new(pid, state.id(), status);
        let credit: ThreadLifecycleTerminationCredit = state.take_termination_credit();
        ZombieThreadTransition {
            zombie: Self { status, state },
            pending: PendingThreadTermination { info, credit },
        }
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the zombie thread.
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
    /// Harvests the zombie thread and reclaims its resources.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the optional kernel stack and user stack of the terminated thread.
    ///
    pub fn harvest(mut self) -> (Option<KernelStack>, Option<UserStack>) {
        (self.state.take_kernel_stack(), self.state.take_user_stack())
    }

    ///
    /// # Description
    ///
    /// Returns the exit status of the zombie thread.
    ///
    /// # Returns
    ///
    /// This function returns the exit status of the terminated thread.
    ///
    pub fn status(&self) -> ExitStatus {
        self.status
    }

    ///
    /// # Description
    ///
    /// Returns whether the zombie thread was detached.
    ///
    /// # Returns
    ///
    /// This function returns `true` if the thread is detached, `false` otherwise.
    ///
    pub fn is_detached(&self) -> bool {
        self.state.is_detached()
    }
}
