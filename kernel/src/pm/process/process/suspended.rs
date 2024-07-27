// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::{
    process::process::{
        interrupted::{
            InterruptReason,
            InterruptedProcess,
        },
        runnable::RunnableProcess,
        state::ProcessState,
    },
    thread::SleepingThread,
};
use ::kcall::ThreadIdentifier;

//==================================================================================================
// Suspended Process
//==================================================================================================

///
/// # Description
///
/// A type that represents a suspended process. A suspended process is a process that has all its
/// threads in either the ready or sleeping states.
///
pub struct SleepingProcess {
    state: ProcessState,
    thread: Option<SleepingThread>,
}

impl SleepingProcess {
    pub fn form_state(process: ProcessState, thread: SleepingThread) -> Self {
        Self {
            state: process,
            thread: Some(thread),
        }
    }

    pub fn state(&self) -> &ProcessState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        &mut self.state
    }

    pub fn terminate(mut self) -> InterruptedProcess {
        let thread = self.thread.take().unwrap();
        let thread = thread.interrupt();
        InterruptedProcess::from_state(self.state, InterruptReason::Killed, thread)
    }

    pub fn wakeup_sleeping_thread(
        self,
        tid: ThreadIdentifier,
    ) -> Result<RunnableProcess, SleepingProcess> {
        match self.thread {
            Some(thread) if thread.id() == tid => {
                let ready_thread = thread.wakeup();
                Ok(RunnableProcess::from_state(self.state, ready_thread))
            },
            _ => Err(self),
        }
    }
}
