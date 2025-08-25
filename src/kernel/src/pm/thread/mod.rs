// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::arch::ContextInformation,
    mm::{
        kstack::KernelStack,
        ustack::UserStack,
    },
};
use ::sys::{
    mm::VirtualAddress,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Modules
//==================================================================================================

mod interrupted;
mod ready;
mod running;
mod sleeping;
mod state;
mod zombie;

//==================================================================================================
// Exports
//==================================================================================================

pub use interrupted::{
    InterruptReason,
    InterruptedThread,
};
pub use ready::ReadyThread;
pub use running::RunningThread;
pub use sleeping::SleepingThread;
pub use zombie::ZombieThread;

//==================================================================================================
// Thread Manager
//==================================================================================================

///
/// # Description
///
/// This structure represents a thread manager that is responsible for creating and managing threads.
///
pub struct ThreadManager {
    /// Next thread identifier to be assigned.
    next_id: ThreadIdentifier,
}

impl ThreadManager {
    ///
    /// # Description
    ///
    /// Creates a new thread manager and initializes the kernel thread.
    ///
    /// # Returns
    ///
    /// This function returns a tuple containing the kernel thread and a new instance of a [`ThreadManager`].
    ///
    fn new() -> (ReadyThread, Self) {
        let kernel: ReadyThread =
            ReadyThread::new(From::<i32>::from(0), None, None, None, ContextInformation::default());
        (
            kernel,
            Self {
                next_id: From::<i32>::from(1),
            },
        )
    }

    ///
    /// # Description
    ///
    /// Creates a new thread with the specified parameters.
    ///
    /// # Parameters
    ///
    /// - `kernel_stack`: Optional kernel stack for the thread.
    /// - `user_stack`: Optional user stack for the thread.
    /// - `user_tda`: Optional base address to user-space thread data area.
    /// - `context`: Execution context for the thread.
    ///
    /// # Returns
    ///
    /// This function returns a new [`ReadyThread`] instance.
    ///
    pub fn create_thread(
        &mut self,
        kernel_stack: Option<KernelStack>,
        user_stack: Option<UserStack>,
        user_tda: Option<VirtualAddress>,
        context: ContextInformation,
    ) -> ReadyThread {
        let id: ThreadIdentifier = self.next_id;
        self.next_id = ThreadIdentifier::from(<i32>::from(self.next_id) + 1);

        ReadyThread::new(id, kernel_stack, user_stack, user_tda, context)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the thread manager.
///
/// # Returns
///
/// This function returns a tuple containing the kernel thread and a new instance of a [`ThreadManager`].
///
pub fn init() -> (ReadyThread, ThreadManager) {
    // TODO: check for double initialization.

    ThreadManager::new()
}
