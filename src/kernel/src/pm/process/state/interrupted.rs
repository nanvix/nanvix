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

    pub fn has_thread(&self, tid: ThreadIdentifier) -> bool {
        // Search in the list of interrupted threads.
        if self
            .interrupted_threads
            .iter()
            .any(|thread| thread.id() == tid)
        {
            return true;
        }

        // Search in the list of sleeping threads.
        if let Some(sleeping_threads) = &self.sleeping_threads {
            if sleeping_threads.iter().any(|thread| thread.id() == tid) {
                return true;
            }
        }

        // Search in the list of zombie threads.
        if let Some(zombie_threads) = &self.zombie_threads {
            if zombie_threads.iter().any(|thread| thread.id() == tid) {
                return true;
            }
        }

        false
    }
}

pub(super) fn interrupt(thread: SleepingThread) -> InterruptedThread {
    thread.interrupt(InterruptReason::Killed)
}
