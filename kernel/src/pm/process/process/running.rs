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
            suspended::SleepingProcess,
            RunnableProcess,
            ZombieProcess,
        },
        thread::RunningThread,
    },
};
use ::alloc::rc::Rc;
use ::core::cell::RefCell;
use ::kcall::ThreadIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a running process.
///
pub struct RunningProcess {
    state: Option<ProcessState>,
    /// Running thread.
    running: Rc<RefCell<Option<RunningThread>>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RunningProcess {
    pub fn from_state(process: ProcessState, running: RunningThread) -> Self {
        Self {
            state: Some(process),
            running: Rc::new(RefCell::new(Some(running))),
        }
    }

    pub fn state(&self) -> &ProcessState {
        self.state.as_ref().unwrap()
    }

    pub fn state_mut(&mut self) -> &mut ProcessState {
        self.state.as_mut().unwrap()
    }

    pub fn schedule(mut self) -> (RunnableProcess, *mut ContextInformation) {
        let running_thread = self.running.borrow_mut().take().unwrap();
        let (ready_thread, ctx) = running_thread.schedule();

        (RunnableProcess::from_state(self.state.take().unwrap(), ready_thread), ctx)
    }

    pub fn sleep(
        mut self,
    ) -> Result<
        (RunnableProcess, *mut ContextInformation),
        (SleepingProcess, *mut ContextInformation),
    > {
        let running_thread = self.running.borrow_mut().take().unwrap();
        let (sleeping_thread, ctx) = running_thread.sleep();

        Err((SleepingProcess::form_state(self.state.take().unwrap(), sleeping_thread), ctx))
    }

    pub fn exit(
        mut self,
        status: i32,
    ) -> Result<(RunnableProcess, *mut ContextInformation), (ZombieProcess, *mut ContextInformation)>
    {
        let running_thread = self.running.borrow_mut().take().unwrap();
        let (zombie_thread, ctx) = running_thread.exit();

        Err((ZombieProcess::new(self.state.take().unwrap(), zombie_thread, status), ctx))
    }

    pub fn get_tid(&self) -> ThreadIdentifier {
        self.running
            .borrow()
            .as_ref()
            .map(|thread| thread.id())
            .unwrap()
    }
}
