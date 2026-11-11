// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

/// Test-only inspection helpers for in-kernel tests.
#[cfg(feature = "test")]
mod test;

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::state::{
        ProcessState,
        RunnableProcess,
    },
    thread::{
        InterruptReason,
        InterruptedThread,
        SleepingThread,
        ThreadRef,
        ThreadRefMut,
        ZombieThread,
    },
};
use ::alloc::{
    boxed::Box,
    collections::vec_deque::VecDeque,
};
use ::sys::pm::ThreadIdentifier;
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Exports
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that was interrupted.
///
#[derive(Debug)]
pub struct InterruptedProcess {
    state: Box<ProcessState>,
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
    interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
    zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
}

impl InterruptedProcess {
    pub(super) fn new(
        process: Box<ProcessState>,
        interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        Self {
            state: process,
            sleeping_threads: None,
            interrupted_threads,
            zombie_threads,
        }
    }

    pub(super) fn from_sleeping(
        process: Box<ProcessState>,
        sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
        interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        Self {
            state: process,
            sleeping_threads,
            interrupted_threads,
            zombie_threads,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub(super) fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    pub fn resume(mut self) -> RunnableProcess {
        let (interrupted_threads, next_thread): (VecDeque<InterruptedThread>, InterruptedThread) =
            self.interrupted_threads.pop_front();
        let ready_thread = next_thread.resume();

        RunnableProcess::from_state(
            self.state,
            NonEmptyVecDeque::new(ready_thread),
            NonEmptyVecDeque::from(interrupted_threads),
            self.sleeping_threads.take(),
            self.zombie_threads.take(),
        )
    }

    ///
    /// # Description
    ///
    /// Terminates an already-interrupted process so that all of its threads exit once resumed.
    ///
    /// Every interrupted thread has its reason overridden to [`InterruptReason::Killed`], and any
    /// remaining sleeping threads are interrupted with the same reason and folded into the
    /// interrupted set (see [`fold_killed`]). Because an interrupted process always retains at
    /// least one interrupted thread, the result is again an [`InterruptedProcess`].
    ///
    /// # Returns
    ///
    /// The interrupted process with all of its threads marked for termination.
    ///
    pub fn terminate(self) -> InterruptedProcess {
        let interrupted_threads: NonEmptyVecDeque<InterruptedThread> =
            kill_interrupted(self.interrupted_threads, self.sleeping_threads);
        InterruptedProcess::new(self.state, interrupted_threads, self.zombie_threads)
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
        // Search in the list of interrupted threads.
        if let Some(thread) = self
            .interrupted_threads
            .iter()
            .find(|thread| thread.id() == tid)
        {
            return Some(ThreadRef::Interrupted(thread));
        }

        // Search in the list of sleeping threads.
        if let Some(sleeping_threads) = &self.sleeping_threads {
            if let Some(thread) = sleeping_threads.iter().find(|thread| thread.id() == tid) {
                return Some(ThreadRef::Sleeping(thread));
            }
        }

        // Search in the list of zombie threads.
        if let Some(zombie_threads) = &self.zombie_threads {
            if let Some(thread) = zombie_threads.iter().find(|thread| thread.id() == tid) {
                return Some(ThreadRef::Zombie(thread));
            }
        }

        None
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
        // Search in the list of interrupted threads.
        if let Some(thread) = self
            .interrupted_threads
            .iter_mut()
            .find(|thread| thread.id() == tid)
        {
            return Some(ThreadRefMut::Interrupted(thread));
        }

        // Search in the list of sleeping threads.
        if let Some(sleeping_threads) = &mut self.sleeping_threads {
            if let Some(thread) = sleeping_threads
                .iter_mut()
                .find(|thread| thread.id() == tid)
            {
                return Some(ThreadRefMut::Sleeping(thread));
            }
        }

        // Search in the list of zombie threads.
        if let Some(zombie_threads) = &mut self.zombie_threads {
            if let Some(thread) = zombie_threads.iter_mut().find(|thread| thread.id() == tid) {
                return Some(ThreadRefMut::Zombie(thread));
            }
        }

        None
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Folds the blocked threads that a process carries into a terminal state into a single set of
/// interrupted threads, every one of which exits once resumed.
///
/// Every already-interrupted thread has its reason overridden to [`InterruptReason::Killed`],
/// whatever it was originally interrupted for, and every sleeping thread is interrupted with that
/// same reason and appended to the set. An interrupted thread carried with any other reason, such
/// as one whose timed wait expired or whose wait was interrupted by a caught signal, would
/// otherwise complete its blocking kernel call and return to user code after its process was
/// terminated or exited.
///
/// Every path that terminates a process, or that exits it, must fold the blocked threads of that
/// process through this function or one of its non-empty variants before any of these threads can
/// be scheduled again.
///
/// # Parameters
///
/// - `interrupted_threads`: Already-interrupted threads of the process.
/// - `sleeping_threads`: Sleeping threads of the process.
///
/// # Returns
///
/// The killed threads, or [`None`] if the process has neither interrupted nor sleeping threads.
///
pub(super) fn fold_killed(
    interrupted_threads: Option<NonEmptyVecDeque<InterruptedThread>>,
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
) -> Option<NonEmptyVecDeque<InterruptedThread>> {
    match interrupted_threads {
        Some(interrupted_threads) => Some(kill_interrupted(interrupted_threads, sleeping_threads)),
        None => sleeping_threads.map(kill_sleeping),
    }
}

///
/// # Description
///
/// Variant of [`fold_killed`] for a process that is known to have interrupted threads.
///
/// # Parameters
///
/// - `interrupted_threads`: Already-interrupted threads of the process.
/// - `sleeping_threads`: Sleeping threads of the process.
///
/// # Returns
///
/// The killed threads.
///
fn kill_interrupted(
    mut interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
    sleeping_threads: Option<NonEmptyVecDeque<SleepingThread>>,
) -> NonEmptyVecDeque<InterruptedThread> {
    // Convert every already-interrupted thread into a killed thread.
    for thread in interrupted_threads.iter_mut() {
        thread.set_killed();
    }

    // Interrupt any sleeping threads with the killed reason and fold them into the set.
    if let Some(sleeping_threads) = sleeping_threads {
        interrupted_threads.append(kill_sleeping(sleeping_threads));
    }

    interrupted_threads
}

///
/// # Description
///
/// Variant of [`fold_killed`] for a process that is known to have sleeping threads only.
///
/// # Parameters
///
/// - `sleeping_threads`: Sleeping threads of the process.
///
/// # Returns
///
/// The killed threads.
///
pub(super) fn kill_sleeping(
    sleeping_threads: NonEmptyVecDeque<SleepingThread>,
) -> NonEmptyVecDeque<InterruptedThread> {
    NonEmptyVecDeque::map(sleeping_threads, |thread: SleepingThread| {
        thread.interrupt(InterruptReason::Killed)
    })
}
