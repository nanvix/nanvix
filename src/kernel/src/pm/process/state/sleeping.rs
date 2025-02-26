// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::state::{
        interrupted::InterruptedProcess,
        runnable::RunnableProcess,
        ProcessState,
    },
    thread::{
        InterruptReason,
        InterruptedThread,
        ReadyThread,
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
// Suspended Process
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that is waiting for a condition to be satisfied.
///
pub struct SleepingProcess {
    state: Box<ProcessState>,
    sleeping_threads: NonEmptyVecDeque<SleepingThread>,
    zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
}

impl SleepingProcess {
    pub(super) fn new(
        process: Box<ProcessState>,
        sleeping_threads: NonEmptyVecDeque<SleepingThread>,
        zombie_threads: Option<NonEmptyVecDeque<ZombieThread>>,
    ) -> Self {
        Self {
            state: process,
            sleeping_threads,
            zombie_threads,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub(super) fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    pub fn terminate(self) -> InterruptedProcess {
        let (mut sleeping_threads, sleeping_thread): (VecDeque<SleepingThread>, SleepingThread) =
            self.sleeping_threads.pop_front();

        let mut interrupted_threads: NonEmptyVecDeque<InterruptedThread> =
            NonEmptyVecDeque::new(sleeping_thread.interrupt(InterruptReason::Killed));

        while let Some(sleeping_thread) = sleeping_threads.pop_front() {
            interrupted_threads.push_back(sleeping_thread.interrupt(InterruptReason::Killed));
        }

        InterruptedProcess::new(self.state, interrupted_threads, self.zombie_threads)
    }

    pub fn wakeup(mut self, tid: ThreadIdentifier) -> Result<RunnableProcess, SleepingProcess> {
        let sleeping_threads: NonEmptyVecDeque<SleepingThread> = self.sleeping_threads;

        // Search for the sleeping thread.
        match sleeping_threads.remove_if(|thread| thread.id() == tid) {
            Ok((sleeping_threads, sleeping_thread)) => {
                let ready_thread: ReadyThread = sleeping_thread.wakeup();
                Ok(RunnableProcess::from_state_with_ready_thread(
                    self.state,
                    NonEmptyVecDeque::new(ready_thread),
                    None,
                    NonEmptyVecDeque::from(sleeping_threads),
                    self.zombie_threads.take(),
                ))
            },
            Err(sleeping_threads) => {
                self.sleeping_threads = sleeping_threads;
                Err(self)
            },
        }
    }

    pub fn add_thread(mut self, ready_thread: ReadyThread) -> RunnableProcess {
        RunnableProcess::from_state_with_ready_thread(
            self.state,
            NonEmptyVecDeque::new(ready_thread),
            None,
            Some(self.sleeping_threads),
            self.zombie_threads.take(),
        )
    }
}
