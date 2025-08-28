// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::{
        x86::cpu::FpuState,
        ContextInformation,
    },
    mm::{
        kstack::KernelStack,
        ustack::UserStack,
    },
    pm::{
        clock,
        sync::condvar::Condvar,
        thread::{
            state::ThreadState,
            RunningThread,
            ZombieThread,
        },
        InterruptReason,
    },
};
use ::alloc::boxed::Box;
use ::core::fmt::Debug;
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
    pm::ThreadIdentifier,
    time::SystemTime,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents a thread that is ready to run.
///
#[derive(Debug)]
pub struct ReadyThread {
    /// Thread state.
    state: Box<ThreadState>,
    /// Time when the thread was admitted to the ready queue.
    admission_time: SystemTime,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ReadyThread {
    ///
    /// # Description
    ///
    /// Creates a new ready thread.
    ///
    /// # Parameters
    ///
    /// - `id`: Thread identifier.
    /// - `kernel_stack`: Optional kernel stack.
    /// - `user_stack`: Optional user stack.
    /// - `user_tda`: Optional base address for the user-space thread data area.
    /// - `context`: Execution context.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a [`ReadyThread`].
    ///
    pub fn new(
        id: ThreadIdentifier,
        kernel_stack: Option<KernelStack>,
        user_stack: Option<UserStack>,
        user_tda: Option<VirtualAddress>,
        context: ContextInformation,
        fpu_state: FpuState,
    ) -> Self {
        Self {
            state: Box::new(ThreadState::new(
                id,
                kernel_stack,
                user_stack,
                user_tda,
                context,
                fpu_state,
            )),
            admission_time: clock::now(),
        }
    }

    ///
    /// # Description
    ///
    /// Creates a ready thread from an existing thread state.
    ///
    /// # Parameters
    ///
    /// - `state`: The thread state.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a [`ReadyThread`].
    ///
    pub fn from_state(state: Box<ThreadState>) -> Self {
        Self {
            state,
            admission_time: clock::now(),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the ready thread.
    ///
    /// # Returns
    ///
    /// This function returns the thread identifier.
    ///
    pub fn id(&self) -> ThreadIdentifier {
        self.state.id()
    }

    ///
    /// # Description
    ///
    /// Transitions the ready thread to running state.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing:
    /// - The running thread.
    /// - An optional interrupt reason.
    /// - A mutable pointer to the execution context
    /// - An optional base address for the user-space thread data area of the running thread.
    ///
    pub fn run(
        mut self,
    ) -> (
        RunningThread,
        Option<InterruptReason>,
        *mut ContextInformation,
        *mut FpuState,
        Option<VirtualAddress>,
    ) {
        let ctx: *mut ContextInformation = self.state.context_mut();
        let fpu_state: *mut FpuState = self.state.fpu_state_mut();
        let interrupt_reason: Option<InterruptReason> = self.state.take_interrupt_reason();
        let user_tda: Option<VirtualAddress> = self.state.get_thread_data_area();
        (RunningThread::from_state(self.state), interrupt_reason, ctx, fpu_state, user_tda)
    }

    ///
    /// # Description
    ///
    /// Terminates the ready thread and transitions it to zombie state.
    ///
    /// # Returns
    ///
    /// This function returns a [`ZombieThread`] instance.
    ///
    pub fn terminate(self) -> ZombieThread {
        ZombieThread::from_state(self.state, ErrorCode::Interrupted.into())
    }

    ///
    /// # Description
    ///
    /// Returns the join condition variable of the ready thread.
    ///
    /// # Returns
    ///
    /// This function returns the join condition variable.
    ///
    pub fn join_cond(&self) -> Condvar {
        self.state.join_cond()
    }

    ///
    /// # Description
    ///
    /// Returns the admission time of the ready thread.
    ///
    /// # Returns
    ///
    /// This function returns the time when the thread was admitted to the ready queue.
    ///
    pub fn admission_time(&self) -> SystemTime {
        self.admission_time
    }
}
