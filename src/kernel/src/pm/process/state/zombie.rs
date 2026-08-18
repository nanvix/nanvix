// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::{
        state::ProcessState,
        LifecycleTerminationCredit,
    },
    thread::{
        ThreadRef,
        ThreadRefMut,
        ZombieThread,
    },
};
use ::alloc::boxed::Box;
use ::sys::{
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
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

/// Process-termination data and capacity transferred at zombie-process creation.
#[must_use]
pub(crate) struct PendingProcessTermination {
    pid: ProcessIdentifier,
    parent: ProcessIdentifier,
    status: ExitStatus,
    credit: LifecycleTerminationCredit,
}

impl PendingProcessTermination {
    ///
    /// # Description
    ///
    /// Splits this pending termination into its metadata and reserved capacity credit.
    ///
    /// # Returns
    ///
    /// A tuple containing the process identifier, parent identifier, exit status, and termination
    /// capacity credit.
    ///
    pub(crate) fn into_parts(
        self,
    ) -> (ProcessIdentifier, ProcessIdentifier, ExitStatus, LifecycleTerminationCredit) {
        (self.pid, self.parent, self.status, self.credit)
    }
}

/// Result of the authoritative transition into zombie-process state.
#[must_use]
pub(crate) struct ZombieProcessTransition {
    zombie: ZombieProcess,
    pending: PendingProcessTermination,
}

impl ZombieProcessTransition {
    ///
    /// # Description
    ///
    /// Creates a zombie process transition and transfers its termination credit.
    ///
    /// # Parameters
    ///
    /// - `process`: Process state that is transitioning to zombie state.
    /// - `zombie_threads`: Non-empty collection of the process's zombie threads.
    /// - `status`: Final exit status of the process.
    ///
    /// # Returns
    ///
    /// A transition containing the zombie process and its pending termination record.
    ///
    /// # Panics
    ///
    /// This function panics if the process does not own a termination credit.
    ///
    pub(super) fn new(
        mut process: Box<ProcessState>,
        zombie_threads: NonEmptyVecDeque<ZombieThread>,
        status: ExitStatus,
    ) -> Self {
        let pid: ProcessIdentifier = process.pid();
        let parent: ProcessIdentifier = process.ppid();
        let credit: LifecycleTerminationCredit = match process.take_termination_credit() {
            Some(credit) => credit,
            None => unreachable!("zombie user process must own a termination credit"),
        };
        Self {
            zombie: ZombieProcess {
                zombie_threads,
                process,
                status,
            },
            pending: PendingProcessTermination {
                pid,
                parent,
                status,
                credit,
            },
        }
    }

    ///
    /// # Description
    ///
    /// Splits this transition into the zombie process and its pending termination record.
    ///
    /// # Returns
    ///
    /// A tuple containing the zombie process and its pending termination record.
    ///
    pub(crate) fn into_parts(self) -> (ZombieProcess, PendingProcessTermination) {
        (self.zombie, self.pending)
    }
}

impl ZombieProcess {
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
    pub fn find_thread(&self, tid: ThreadIdentifier) -> Option<ThreadRef<'_>> {
        self.zombie_threads
            .iter()
            .find(|thread| thread.id() == tid)
            .map(ThreadRef::Zombie)
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
    pub fn find_thread_mut(&mut self, tid: ThreadIdentifier) -> Option<ThreadRefMut<'_>> {
        self.zombie_threads
            .iter_mut()
            .find(|thread| thread.id() == tid)
            .map(ThreadRefMut::Zombie)
    }
}
