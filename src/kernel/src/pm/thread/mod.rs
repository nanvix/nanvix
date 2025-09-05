// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![cfg_attr(not(feature = "sse"), allow(dead_code))]

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
    pm::thread::state::ThreadState,
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
// Thread Reference
//==================================================================================================

///
/// # Description
///
/// A reference to a thread.
///
pub enum ThreadRef<'a> {
    /// A reference to a ready thread.
    Ready(&'a ReadyThread),
    /// A reference to a running thread.
    Running(&'a RunningThread),
    /// A reference to a sleeping thread.
    Sleeping(&'a SleepingThread),
    /// A reference to an interrupted thread.
    Interrupted(&'a InterruptedThread),
    /// A reference to a zombie thread.
    Zombie(&'a ZombieThread),
}

impl<'a> ThreadRef<'a> {
    ///
    /// # Description
    ///
    /// Returns a reference to the thread's state.
    ///
    /// # Return Value
    ///
    /// This function returns a reference to the thread's state.
    ///
    #[allow(dead_code)] // TODO: remove this.
    pub fn thread_state(&self) -> &ThreadState {
        match self {
            ThreadRef::Ready(thread) => thread.thread_state(),
            ThreadRef::Running(thread) => thread.thread_state(),
            ThreadRef::Sleeping(thread) => thread.thread_state(),
            ThreadRef::Interrupted(thread) => thread.thread_state(),
            ThreadRef::Zombie(thread) => thread.thread_state(),
        }
    }
}

//==================================================================================================
// Mutable Thread Reference
//==================================================================================================

///
/// # Description
///
/// A mutable reference to a thread.
///
pub enum ThreadRefMut<'a> {
    /// A mutable reference to a ready thread.
    Ready(&'a mut ReadyThread),
    /// A mutable reference to a running thread.
    Running(&'a mut RunningThread),
    /// A mutable reference to a sleeping thread.
    Sleeping(&'a mut SleepingThread),
    /// A mutable reference to an interrupted thread.
    Interrupted(&'a mut InterruptedThread),
    /// A mutable reference to a zombie thread.
    Zombie(&'a mut ZombieThread),
}

impl<'a> ThreadRefMut<'a> {
    ///
    /// # Description
    ///
    /// Returns a mutable reference to the thread's state.
    ///
    /// # Return Value
    ///
    /// This function returns a mutable reference to the thread's state.
    ///
    pub fn thread_state_mut(&mut self) -> &mut ThreadState {
        match self {
            ThreadRefMut::Ready(thread) => thread.thread_state_mut(),
            ThreadRefMut::Running(thread) => thread.thread_state_mut(),
            ThreadRefMut::Sleeping(thread) => thread.thread_state_mut(),
            ThreadRefMut::Interrupted(thread) => thread.thread_state_mut(),
            ThreadRefMut::Zombie(thread) => thread.thread_state_mut(),
        }
    }
}

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
        let kernel: ReadyThread = {
            ReadyThread::new(
                From::<i32>::from(0),
                None,
                None,
                None,
                ContextInformation::default(),
                // SAFETY: calls to FpuState::new are synchronized.
                unsafe { FpuState::new() },
            )
        };
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

        ReadyThread::new(
            id,
            kernel_stack,
            user_stack,
            user_tda,
            context,
            // SAFETY: calls to FpuState::new are synchronized.
            unsafe { FpuState::new() },
        )
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
