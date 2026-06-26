// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    /// interrupted set. Because an interrupted process always retains at least one interrupted
    /// thread, the result is again an [`InterruptedProcess`].
    ///
    /// # Returns
    ///
    /// The interrupted process with all of its threads marked for termination.
    ///
    pub fn terminate(mut self) -> InterruptedProcess {
        // Convert every already-interrupted thread into a killed thread.
        for thread in self.interrupted_threads.iter_mut() {
            thread.set_killed();
        }

        // Interrupt any remaining sleeping threads with the killed reason and fold them into the
        // interrupted set.
        if let Some(sleeping_threads) = self.sleeping_threads.take() {
            let killed_threads: NonEmptyVecDeque<InterruptedThread> =
                NonEmptyVecDeque::map(sleeping_threads, interrupt);
            self.interrupted_threads.append(killed_threads);
        }

        self
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

pub(super) fn interrupt(thread: SleepingThread) -> InterruptedThread {
    thread.interrupt(InterruptReason::Killed)
}
