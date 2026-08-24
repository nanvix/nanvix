// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::state::ProcessState,
    thread::{
        ThreadRef,
        ThreadRefMut,
        ZombieThread,
    },
};
use ::alloc::boxed::Box;
use ::sys::{
    pm::ThreadIdentifier,
    ExitStatus,
};
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Zombie Process
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that finished its execution and is waiting for its parent
/// to collect its exit status and release its resources.
///
#[derive(Debug)]
pub struct ZombieProcess {
    zombie_threads: NonEmptyVecDeque<ZombieThread>,
    process: Box<ProcessState>,
    status: ExitStatus,
}

impl ZombieProcess {
    pub(super) fn new(
        process: Box<ProcessState>,
        zombie_threads: NonEmptyVecDeque<ZombieThread>,
        status: ExitStatus,
    ) -> Self {
        Self {
            zombie_threads,
            process,
            status,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.process
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.process
    }

    pub fn bury(self) -> (NonEmptyVecDeque<ZombieThread>, Box<ProcessState>, ExitStatus) {
        (self.zombie_threads, self.process, self.status)
    }

    ///
    /// # Description
    ///
    /// Finds a thread in the target process.
    ///
    /// # Arguments
    ///
    /// - `tid`: Identifier of the thread to find.
    ///
    /// # Returns
    ///
    /// If a thread that matches the specified thread identifier is found, then a reference to it is
    /// returned. Otherwise, empty is returned instead.
    ///
    // `map` uses an explicit closure instead of the `ThreadRef::Zombie`
    // constructor as a bare function value, which the Verus frontend cannot lower.
    #[allow(clippy::redundant_closure)]
    pub fn find_thread(&self, tid: ThreadIdentifier) -> Option<ThreadRef<'_>> {
        self.zombie_threads
            .iter()
            .find(|thread| thread.id() == tid)
            .map(|thread| ThreadRef::Zombie(thread))
    }

    ///
    /// # Description
    ///
    /// Finds a thread in the target process.
    ///
    /// # Arguments
    ///
    /// - `tid`: Identifier of the thread to find.
    ///
    /// # Returns
    ///
    /// If a thread that matches the specified thread identifier is found, then a mutable reference
    /// to it is returned. Otherwise, empty is returned instead.
    ///
    // `map` uses an explicit closure instead of the `ThreadRefMut::Zombie`
    // constructor as a bare function value, which the Verus frontend cannot lower.
    #[allow(clippy::redundant_closure)]
    pub fn find_thread_mut(&mut self, tid: ThreadIdentifier) -> Option<ThreadRefMut<'_>> {
        self.zombie_threads
            .iter_mut()
            .find(|thread| thread.id() == tid)
            .map(|thread| ThreadRefMut::Zombie(thread))
    }
}
