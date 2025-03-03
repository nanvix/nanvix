// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    mm::ustack::UserStack,
};
use ::alloc::boxed::Box;
use ::core::{
    fmt::Debug,
    pin::Pin,
};
use ::sys::{
    error::ErrorCode,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Thread
//==================================================================================================

#[derive(Debug)]
pub struct Thread {
    /// Thread identifier.
    id: ThreadIdentifier,
    /// User stack.
    user_stack: Option<UserStack>,
    /// Execution context.
    context: Pin<Box<ContextInformation>>,
}

impl Thread {
    pub fn new(
        id: ThreadIdentifier,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> Self {
        Self {
            id,
            context: Box::pin(context),
            user_stack,
        }
    }

    fn context_mut(&mut self) -> *mut ContextInformation {
        self.context.as_mut().get_mut() as *mut ContextInformation
    }
}

//==================================================================================================
// Running Thread
//==================================================================================================

pub struct RunningThread(Thread);

impl RunningThread {
    pub fn sleep(mut self) -> (SleepingThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (SleepingThread(self.0), ctx)
    }

    pub fn schedule(mut self) -> (ReadyThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (ReadyThread(self.0), ctx)
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the target thread.
    ///
    /// # Returns
    ///
    /// The identifier of the target thread.
    ///
    pub fn id(&self) -> ThreadIdentifier {
        self.0.id
    }

    pub fn exit(mut self, status: usize) -> (ZombieThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (
            ZombieThread {
                state: self.0,
                status,
            },
            ctx,
        )
    }
}

//==================================================================================================
// Ready Thread
//==================================================================================================

pub struct ReadyThread(Thread);

impl ReadyThread {
    pub fn new(
        id: ThreadIdentifier,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> Self {
        Self(Thread::new(id, user_stack, context))
    }

    pub fn tid(&self) -> ThreadIdentifier {
        self.0.id
    }

    pub fn resume(mut self) -> (RunningThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (RunningThread(self.0), ctx)
    }

    pub fn terminate(self) -> ZombieThread {
        ZombieThread {
            state: self.0,
            status: ErrorCode::Interrupted.into_errno() as usize,
        }
    }
}

//==================================================================================================
// Sleeping Thread
//==================================================================================================

pub struct SleepingThread(Thread);

impl SleepingThread {
    pub fn wakeup(self) -> ReadyThread {
        ReadyThread(self.0)
    }

    pub fn interrupt(self, reason: InterruptReason) -> InterruptedThread {
        InterruptedThread {
            thread: self.0,
            reason,
        }
    }

    pub fn id(&self) -> ThreadIdentifier {
        self.0.id
    }
}

//==================================================================================================
// Interrupted Thread
//==================================================================================================

#[derive(Debug, PartialEq, Eq)]
pub enum InterruptReason {
    /// Process was killed.
    Killed,
}
pub struct InterruptedThread {
    thread: Thread,
    reason: InterruptReason,
}

impl InterruptedThread {
    pub fn tid(&self) -> ThreadIdentifier {
        self.thread.id
    }

    pub fn resume(mut self) -> (RunningThread, InterruptReason, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.thread.context_mut();
        (RunningThread(self.thread), self.reason, ctx)
    }
}

//==================================================================================================
// Zombie Thread
//==================================================================================================

#[allow(unused)]
pub struct ZombieThread {
    status: usize,
    state: Thread,
}

impl ZombieThread {
    pub fn tid(&self) -> ThreadIdentifier {
        self.state.id
    }

    pub fn harvest(mut self) -> Option<UserStack> {
        self.state.user_stack.take()
    }

    pub fn status(&self) -> usize {
        self.status
    }
}

//==================================================================================================
// Thread Manager
//==================================================================================================

pub struct ThreadManager {
    next_id: ThreadIdentifier,
}

impl ThreadManager {
    fn new() -> (ReadyThread, Self) {
        let kernel: ReadyThread =
            ReadyThread::new(ThreadIdentifier::from(0), None, ContextInformation::default());
        (
            kernel,
            Self {
                next_id: ThreadIdentifier::from(1),
            },
        )
    }

    pub fn create_thread(
        &mut self,
        user_stack: Option<UserStack>,
        context: ContextInformation,
    ) -> ReadyThread {
        let id: ThreadIdentifier = self.next_id;
        self.next_id = ThreadIdentifier::from(Into::<usize>::into(self.next_id) + 1);

        ReadyThread(Thread::new(id, user_stack, context))
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Initializes the thread manager.
pub fn init() -> (ReadyThread, ThreadManager) {
    // TODO: check for double initialization.

    ThreadManager::new()
}
