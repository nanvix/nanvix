// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::ContextInformation;
use ::alloc::boxed::Box;
use ::core::{
    fmt::Debug,
    pin::Pin,
};
use ::sys::pm::ThreadIdentifier;

//==================================================================================================
// Thread
//==================================================================================================

#[derive(Debug)]
struct Thread {
    /// Thread identifier.
    id: ThreadIdentifier,
    /// Execution context.
    context: Pin<Box<ContextInformation>>,
}

impl Thread {
    pub fn new(id: ThreadIdentifier, context: ContextInformation) -> Self {
        Self {
            id,
            context: Box::pin(context),
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

    pub fn exit(mut self) -> (ZombieThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (ZombieThread(self.0), ctx)
    }
}

//==================================================================================================
// Ready Thread
//==================================================================================================

pub struct ReadyThread(Thread);

impl ReadyThread {
    pub fn new(id: ThreadIdentifier, context: ContextInformation) -> Self {
        Self(Thread::new(id, context))
    }

    pub fn resume(mut self) -> (RunningThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (RunningThread(self.0), ctx)
    }

    pub fn terminate(self) -> ZombieThread {
        ZombieThread(self.0)
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
    pub fn resume(mut self) -> (RunningThread, InterruptReason, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.thread.context_mut();
        (RunningThread(self.thread), self.reason, ctx)
    }
}

//==================================================================================================
// Zombie Thread
//==================================================================================================

#[allow(unused)]
pub struct ZombieThread(Thread);

//==================================================================================================
// Thread Manager
//==================================================================================================

pub struct ThreadManager {
    next_id: ThreadIdentifier,
}

impl ThreadManager {
    fn new() -> (ReadyThread, Self) {
        let kernel: ReadyThread =
            ReadyThread::new(ThreadIdentifier::from(0), ContextInformation::default());
        (
            kernel,
            Self {
                next_id: ThreadIdentifier::from(1),
            },
        )
    }

    pub fn create_thread(&mut self, context: ContextInformation) -> ReadyThread {
        let id: ThreadIdentifier = self.next_id;
        self.next_id = ThreadIdentifier::from(Into::<usize>::into(self.next_id) + 1);

        ReadyThread(Thread::new(id, context))
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
