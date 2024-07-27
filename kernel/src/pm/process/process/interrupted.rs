// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    pm::{
        process::process::{
            state::ProcessState,
            RunningProcess,
        },
        thread::InterruptedThread,
    },
};

//==================================================================================================
// Exports
//==================================================================================================

pub enum InterruptReason {
    /// Process was killed.
    Killed,
}
pub struct InterruptedProcess {
    state: ProcessState,
    reason: InterruptReason,
    thread: Option<InterruptedThread>,
}

impl InterruptedProcess {
    pub fn from_state(
        process: ProcessState,
        reason: InterruptReason,
        thread: InterruptedThread,
    ) -> Self {
        Self {
            state: process,
            reason,
            thread: Some(thread),
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    pub fn resume(mut self) -> (RunningProcess, InterruptReason, *mut ContextInformation) {
        let thread = self.thread.take().unwrap();
        let (thread, ctx) = thread.resume();
        (RunningProcess::from_state(self.state, thread), self.reason, ctx)
    }
}
