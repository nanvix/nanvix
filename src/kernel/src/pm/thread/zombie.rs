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
    pm::thread::state::ThreadState,
};
use ::alloc::boxed::Box;
use ::core::fmt::Debug;
use ::sys::{
    pm::ThreadIdentifier,
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
    /// - `state`: The thread state.
    /// - `status`: The exit status of the terminated thread.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a [`ZombieThread`].
    ///
    pub(super) fn from_state(state: Box<ThreadState>, status: ExitStatus) -> Self {
        Self { status, state }
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
    pub fn harvest(self) -> (Option<KernelStack>, Option<UserStack>) {
        let mut this = self;
        (this.state.take_kernel_stack(), this.state.take_user_stack())
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
