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
        },
    },
};
use ::alloc::collections::vec_deque::VecDeque;
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
    state: ProcessState,
    interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
}

impl InterruptedProcess {
    pub(super) fn new(
        process: ProcessState,
        interrupted_threads: NonEmptyVecDeque<InterruptedThread>,
    ) -> Self {
        Self {
            state: process,
            interrupted_threads,
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub(super) fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    pub fn resume(self) -> (RunningProcess, InterruptReason, *mut ContextInformation) {
        let (interrupted_threads, next_thread): (VecDeque<InterruptedThread>, InterruptedThread) =
            self.interrupted_threads.pop_front();
        let (thread, reason, ctx) = next_thread.resume();
        (
            RunningProcess::new(
                self.state,
                thread,
                None,
                NonEmptyVecDeque::from(interrupted_threads),
                None,
                None,
            ),
            reason,
            ctx,
        )
    }
}

pub(super) fn interrupt(thread: SleepingThread) -> InterruptedThread {
    thread.interrupt(InterruptReason::Killed)
}
