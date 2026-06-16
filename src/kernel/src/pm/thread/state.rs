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
        sync::{
            condvar::Condvar,
            mutex::MutexGuard,
        },
        InterruptReason,
    },
};
use ::alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    fmt,
};
use ::core::pin::Pin;
use ::sys::{
    error::Error,
    mm::VirtualAddress,
    pm::{
        MutexAddress,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents the state of a thread.
///
pub struct ThreadState {
    /// Thread identifier.
    id: ThreadIdentifier,
    /// Kernel stack.
    kernel_stack: Option<KernelStack>,
    /// User stack.
    user_stack: Option<UserStack>,
    /// Condition variable for join.
    join_cond: Condvar,
    /// Execution context.
    context: Pin<Box<ContextInformation>>,
    /// Optional base address for the user-space thread data area.
    user_tda: Option<VirtualAddress>,
    /// Lookup table of locked mutexes.
    locked_mutexes: BTreeMap<MutexAddress, MutexGuard>,
    /// Interrupt reason, if any.
    interrupt_reason: Option<InterruptReason>,
    /// FPU state.
    fpu_state: Pin<Box<FpuState>>,
    /// Whether the thread is detached (will be auto-harvested on exit).
    detached: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ThreadState {
    ///
    /// # Description
    ///
    /// Creates a new thread state.
    ///
    /// # Parameters
    ///
    /// - `id`: Thread identifier.
    /// - `kernel_stack`: Optional kernel stack.
    /// - `user_stack`: Optional user stack.
    /// - `user_tda`: Optional base address for the user-space user-space thread data area.
    /// - `context`: Execution context.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a [`ThreadState`].
    ///
    pub(super) fn new(
        id: ThreadIdentifier,
        kernel_stack: Option<KernelStack>,
        user_stack: Option<UserStack>,
        user_tda: Option<VirtualAddress>,
        context: ContextInformation,
        fpu_state: FpuState,
    ) -> Self {
        Self {
            id,
            context: Box::pin(context),
            kernel_stack,
            user_stack,
            user_tda,
            join_cond: Condvar::new(),
            locked_mutexes: BTreeMap::new(),
            interrupt_reason: None,
            fpu_state: Box::pin(fpu_state),
            detached: false,
        }
    }

    ///
    /// # Description
    ///
    /// Returns a mutable pointer to the execution context of the thread.
    ///
    /// # Returns
    ///
    /// This function returns a mutable pointer to the execution context of the thread.
    ///
    pub(super) fn context_mut(&mut self) -> *mut ContextInformation {
        self.context.as_mut().get_mut() as *mut ContextInformation
    }

    ///
    /// # Description
    ///
    /// Returns a mutable pointer to the FPU state of the thread.
    ///
    /// # Returns
    ///
    /// This function returns a mutable pointer to the FPU state of the thread.
    ///
    pub fn fpu_state_mut(&mut self) -> *mut FpuState {
        self.fpu_state.as_mut().get_mut() as *mut FpuState
    }

    ///
    /// # Description
    ///
    /// Returns the identifier of the thread.
    ///
    /// # Returns
    ///
    /// This function returns the identifier of the thread.
    ///
    pub(super) fn id(&self) -> ThreadIdentifier {
        self.id
    }

    ///
    /// # Description
    ///
    /// Returns the join condition variable of the thread.
    ///
    /// # Returns
    ///
    /// This function returns the join condition variable of the thread.
    ///
    pub(super) fn join_cond(&self) -> Condvar {
        self.join_cond.clone()
    }

    ///
    /// # Description
    ///
    /// Stores a mutex guard in the thread state.
    ///
    /// # Parameters
    ///
    /// - `address`: The address of the mutex.
    /// - `guard`: The mutex guard to store.
    ///
    pub(super) fn store_mutex_guard(&mut self, address: MutexAddress, guard: MutexGuard) {
        self.locked_mutexes.insert(address, guard);
    }

    ///
    /// # Description
    ///
    /// Returns the mutex guard associated with the thread.
    ///
    /// # Parameters
    ///
    /// - `address`: The address of the mutex.
    ///
    /// # Returns
    ///
    /// This function returns the mutex guard associated with the thread, if any.
    ///
    pub(super) fn take_mutex_guard(&mut self, address: MutexAddress) -> Option<MutexGuard> {
        self.locked_mutexes.remove(&address)
    }

    ///
    /// # Description
    ///
    /// Sets reason why a thread was interrupted.
    ///
    /// # Parameters
    ///
    /// - `reason`: The reason for the interruption.
    ///
    pub(super) fn set_interrupt_reason(&mut self, reason: InterruptReason) {
        self.interrupt_reason = Some(reason);
    }

    ///
    /// # Description
    ///
    /// Returns the interrupt reason, if any.
    ///
    /// # Returns
    ///
    /// This function returns the interrupt reason, if any.
    ///
    pub(super) fn take_interrupt_reason(&mut self) -> Option<InterruptReason> {
        self.interrupt_reason.take()
    }

    ///
    /// # Description
    ///
    /// Returns whether the thread is detached.
    ///
    pub(super) fn is_detached(&self) -> bool {
        self.detached
    }

    ///
    /// # Description
    ///
    /// Marks the thread as detached. A detached thread is auto-harvested when it exits.
    ///
    pub(super) fn set_detached(&mut self) {
        self.detached = true;
    }

    ///
    /// # Description
    ///
    /// Returns the kernel stack of the thread, if any.
    ///
    /// # Returns
    ///
    /// This function returns the kernel stack of the thread, if any.
    ///
    pub(super) fn take_kernel_stack(&mut self) -> Option<KernelStack> {
        self.kernel_stack.take()
    }

    ///
    /// # Description
    ///
    /// Checks the guard watermark of the kernel stack for corruption.
    ///
    /// # Returns
    ///
    /// Upon success (watermark intact or no kernel stack), `Ok(())` is returned. Upon failure
    /// (watermark corrupted), an error is returned.
    ///
    pub(super) fn check_guard_watermark(&self) -> Result<(), Error> {
        if let Some(ref kstack) = self.kernel_stack {
            kstack.check_guard_watermark()
        } else {
            Ok(())
        }
    }

    ///
    /// # Description
    ///
    /// Returns the guard threshold of the kernel stack, if any.
    ///
    /// # Returns
    ///
    /// The guard threshold value, or `None` if this thread has no kernel stack.
    ///
    #[cfg(feature = "exception-stack-guard")]
    pub(super) fn guard_threshold(&self) -> Option<u32> {
        self.kernel_stack.as_ref().map(|ks| ks.guard_threshold())
    }

    ///
    /// # Description
    ///
    /// Returns the user stack of the thread, if any.
    ///
    /// # Returns
    ///
    /// This function returns the user stack of the thread, if any.
    ///
    pub(super) fn take_user_stack(&mut self) -> Option<UserStack> {
        self.user_stack.take()
    }

    ///
    /// # Description
    ///
    /// Sets the base address for the user-space thread data area in the target thread state.
    ///
    /// # Parameters
    ///
    /// - `user_tda`: Optional thread data area pointer to set.
    ///
    pub(super) fn store_thread_data_area(&mut self, user_tda: Option<VirtualAddress>) {
        self.user_tda = user_tda;
    }

    ///
    /// # Description
    ///
    /// Returns the base address for the user-space thread data area stored in the target thread
    /// state.
    ///
    /// # Returns
    ///
    /// This function returns the base address for the user-space thread data area stored in the
    /// target thread state.
    ///
    pub(super) fn get_thread_data_area(&self) -> Option<VirtualAddress> {
        self.user_tda
    }
}

impl fmt::Debug for ThreadState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Thread {{ id: {:?} }}", self.id)
    }
}

impl Drop for ThreadState {
    fn drop(&mut self) {
        if !self.locked_mutexes.is_empty() {
            error!(
                "drop(): dropping thread state with locked mutexes (self.id={:?}, \
                 self.locked_mutexes={:?})",
                self.id, self.locked_mutexes
            );
        }
    }
}
