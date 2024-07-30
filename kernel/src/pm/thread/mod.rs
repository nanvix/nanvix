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
use ::sys::{
    error::Error,
    pm::ThreadIdentifier,
};

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

    pub fn interrupt(self) -> InterruptedThread {
        InterruptedThread(self.0)
    }

    pub fn id(&self) -> ThreadIdentifier {
        self.0.id
    }
}

//==================================================================================================
// Interrupted Thread
//==================================================================================================

pub struct InterruptedThread(Thread);

impl InterruptedThread {
    pub fn resume(mut self) -> (RunningThread, *mut ContextInformation) {
        let ctx: *mut ContextInformation = self.0.context_mut();
        (RunningThread(self.0), ctx)
    }
}

//==================================================================================================
// Zombie Thread
//==================================================================================================

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

    pub fn create_thread(&mut self, context: ContextInformation) -> Result<ReadyThread, Error> {
        let id: ThreadIdentifier = self.next_id;
        self.next_id = ThreadIdentifier::from(Into::<usize>::into(self.next_id) + 1);

        Ok(ReadyThread(Thread::new(id, context)))
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
