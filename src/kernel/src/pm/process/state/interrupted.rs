// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    pm::{
        process::state::{
            ProcessState,
            RunningProcess,
        },
        thread::{
            InterruptReason,
            InterruptedThread,
            SleepingThread,
            ZombieThread,
        },
    },
};
use ::alloc::{
    boxed::Box,
    collections::vec_deque::VecDeque,
};
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Exports
//==================================================================================================

///
/// # Description
///
/// A type that represents a process that was interrupted.
///
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

    pub fn resume(mut self) -> (RunningProcess, InterruptReason, *mut ContextInformation) {
        let (interrupted_threads, next_thread): (VecDeque<InterruptedThread>, InterruptedThread) =
            self.interrupted_threads.pop_front();
        let (thread, reason, ctx) = next_thread.resume();
        (
            RunningProcess::new(
                self.state,
                thread,
                None,
                NonEmptyVecDeque::from(interrupted_threads),
                self.sleeping_threads.take(),
                self.zombie_threads.take(),
            ),
            reason,
            ctx,
        )
    }
}

pub(super) fn interrupt(thread: SleepingThread) -> InterruptedThread {
    thread.interrupt(InterruptReason::Killed)
}
