// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::{
        arch::ContextInformation,
        mem::{
            Address,
            PageAligned,
            VirtualAddress,
        },
        platform::Interrupts,
    },
    mm::{
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        process::{
            manager::{
                ProcessManager,
                ProcessManagerInner,
            },
            state::RunningProcess,
        },
        sync::{
            condvar::Condvar,
            mutex::{
                Mutex,
                MutexGuard,
            },
        },
        thread::{
            InterruptReason,
            ReadyThread,
            ThreadManager,
            ZombieThread,
        },
        SleepError,
        ORDER,
    },
    PERF_SCHED_EXIT_CONTEXT_SWITCHES,
    PERF_SCHED_EXIT_THREAD_CONTEXT_SWITCHES,
    PERF_SCHED_GIVEUP_CONTEXT_SWITCHES,
    PERF_SCHED_HARD_CONTEXT_SWITCHES,
    PERF_SCHED_KERNEL_IDLE,
    PERF_SCHED_SLEEP_CONTEXT_SWITCHES,
    PERF_SCHED_SOFT_CONTEXT_SWITCHES,
    PERF_SCHED_WAKEUP,
};
use ::alloc::rc::Rc;
use ::arch::mem::PAGE_SIZE;
use ::config::kernel::SCHEDULER_FREQ;
use ::core::{
    cell::{
        RefCell,
        RefMut,
    },
    hint::{
        cold_path,
        unlikely,
    },
    sync::atomic::{
        AtomicI32,
        AtomicUsize,
    },
};
use ::sys::{
    error::Error,
    ipc::Message,
    pm::{
        ConditionAddress,
        MutexAddress,
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
    ExitStatus,
};

//==================================================================================================
// Global Variables
//==================================================================================================

/// Process manager.
static mut PROCESS_MANAGER: Option<ProcessManager> = None;

/// ID of the current process.
static CURRENT_PID: AtomicI32 = AtomicI32::new(ProcessIdentifier::KERNEL_RAW);

/// ID of the current thread.
pub(super) static CURRENT_TID: AtomicI32 = AtomicI32::new(ProcessIdentifier::KERNEL_RAW);

/// Remaining quantum for the current thread.
static REMAINING_QUANTUM: AtomicUsize = AtomicUsize::new(SCHEDULER_FREQ);

/// ID of thread that owns the FPU.
pub(super) static FPU_OWNER_TID: AtomicI32 = AtomicI32::new(ThreadIdentifier::KERNEL_RAW);

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessManager {
    ///
    /// # Description
    ///
    /// Initializes the process manager.
    ///
    /// # Parameters
    ///
    /// - `interrupt_capable`: Indicates whether the process manager is interrupt capable.
    /// - `kernel`: Kernel process.
    /// - `root`: Root virtual memory.
    /// - `tm`: Thread manager.
    ///
    /// # Returns
    ///
    /// A handle to the process manager is returned.
    ///
    pub fn init(
        interrupt_capable: bool,
        kernel: ReadyThread,
        root: Vmem,
        tm: ThreadManager,
    ) -> ProcessManager {
        // Check if the process manager is already initialized.
        if unlikely(unsafe { PROCESS_MANAGER.is_some() }) {
            panic!("process manager was already initialized");
        }

        let pm: Rc<RefCell<ProcessManagerInner>> =
            Rc::new(RefCell::new(ProcessManagerInner::new(interrupt_capable, kernel, root, tm)));

        // SAFETY: This happens during kernel initialization and no other threads are running.
        unsafe { PROCESS_MANAGER = Some(ProcessManager(pm.clone())) };

        ProcessManager(pm)
    }

    ///
    /// # Description
    ///
    /// Gets a reference to the process manager.
    ///
    /// # Safety
    ///
    /// This function panics if the process manager is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if an only if the following conditions are met:
    ///
    /// - The process manager is initialized.
    ///
    pub unsafe fn get<'a>() -> &'a ProcessManager {
        if let Some(ref pm) = PROCESS_MANAGER {
            pm
        } else {
            cold_path();
            panic!("process manager is not initialized");
        }
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the process manager.
    ///
    /// # Safety
    ///
    /// This function panics if the process manager is not initialized.
    ///
    /// This function is unsafe because it operates on a global variable.
    ///
    /// This function is safe to use if an only if the following conditions are met:
    ///
    /// - The process manager is initialized.
    ///
    pub unsafe fn get_mut<'a>() -> &'a mut ProcessManager {
        if let Some(ref mut pm) = PROCESS_MANAGER {
            pm
        } else {
            cold_path();
            panic!("process manager is not initialized");
        }
    }

    ///
    /// # Description
    ///
    /// Terminates the calling process.
    ///
    /// # Parameters
    ///
    /// - `status`: Exit status.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function does not return. The current thread terminates,
    /// and any other threads in the calling process are scheduled for termination. Once all threads
    /// in the process have terminated, the kernel will clean up any resources associated with the
    /// process.  If an error occurs, an error code is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it may terminate the calling thread.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The calling thread is not a kernel thread.
    /// - The process manager is initialized.
    /// - The calling thread does not hold a reference to the process manager.
    /// - Access to the process manager is synchronized.
    /// - The processor is running with interrupts disabled.
    /// - The processor is running in privileged mode.
    ///
    pub unsafe fn exit(status: ExitStatus) -> Result<!, Error> {
        trace!("status={status:?}");

        // Terminate the calling process and select another process to run next.
        let (next_pid, next_tid, from, to, user_tda): (
            ProcessIdentifier,
            ThreadIdentifier,
            *mut ContextInformation,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = Self::get_mut().try_borrow_mut()?.exit(status);

        // SAFETY: `from` and `to` point to valid context information structures, and the processor
        // is running with interrupts disabled.
        PERF_SCHED_EXIT_CONTEXT_SWITCHES.fetch_add(1, ORDER);
        Self::switch(next_pid, next_tid, from, to, user_tda);

        // SAFETY: Self::switch() performs a context switch and never returns. If this line is ever
        // reached, it indicates a critical bug and undefined behavior. This is considered
        // unreachable by design.
        core::hint::unreachable_unchecked()
    }

    ///
    /// # Description
    ///
    /// Terminates the calling thread.
    ///
    /// # Parameters
    ///
    /// - `status`: Exit status.
    ///
    /// # Returns
    ///
    /// This function does not return on success: the current thread is terminated, and its exit
    /// status is made available to any threads waiting for it. If the calling thread is the last
    /// thread in its process, the process is also terminated and all associated resources are
    /// cleaned up. If an error occurs, an error code is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it may terminate the calling thread.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The calling thread is not a kernel thread.
    /// - The process manager is initialized.
    /// - The calling thread does not hold a reference to the process manager.
    /// - Access to the process manager is synchronized.
    /// - The processor is running with interrupts disabled.
    /// - The processor is running in privileged mode.
    ///
    pub unsafe fn exit_thread(status: ExitStatus) -> Result<!, Error> {
        // Terminate the calling thread and select another thread to run next.
        let (next_pid, next_tid, from, to, user_tda): (
            ProcessIdentifier,
            ThreadIdentifier,
            *mut ContextInformation,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = {
            // Create a scope so the join condition variable is dropped before we context switch.
            // If we do not do this, the condition variable the reference count for the condition
            // variable will not be decremented, causing a memory leak.

            let (next_pid, next_tid, join_cond, from, to, user_tda): (
                ProcessIdentifier,
                ThreadIdentifier,
                Condvar,
                *mut ContextInformation,
                *mut ContextInformation,
                Option<VirtualAddress>,
            ) = Self::get_mut().try_borrow_mut()?.exit_thread(status);

            join_cond.notify_all()?;

            (next_pid, next_tid, from, to, user_tda)
        };

        // SAFETY: `from` and `to` point to valid context information structures, and the processor
        // is running with interrupts disabled.
        PERF_SCHED_EXIT_THREAD_CONTEXT_SWITCHES.fetch_add(1, ORDER);
        Self::switch(next_pid, next_tid, from, to, user_tda);

        // SAFETY: Self::switch() performs a context switch and never returns. If this line is ever
        // reached, it indicates a critical bug and undefined behavior. This is considered
        // unreachable by design.
        core::hint::unreachable_unchecked()
    }

    ///
    /// # Description
    ///
    /// Joins a thread.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier in which the thread is running.
    /// - `tid`: Thread identifier of the thread to join.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the status of the thread is returned. Otherwise, an error code is
    /// returned instead.
    ///
    /// # Safety
    ///
    /// This function panics if the kernel process tries to sleep.
    ///
    /// This function is unsafe because it blocks the calling thread until it is woken up by another
    /// thread.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    /// - This function is invoked without holding any resources.
    /// - The process manager is initialized.
    /// - Access to the process manager is synchronized.
    /// - The memory manager is initialized.
    /// - Access to the memory manager is synchronized.
    ///
    pub unsafe fn join_thread(
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
    ) -> Result<ExitStatus, SleepError> {
        trace!("pid={:?}, tid={:?}", pid, tid);

        loop {
            let result: Result<ZombieThread, Result<Condvar, Error>> = Self::get_mut()
                .try_borrow_mut()
                .map_err(SleepError::Generic)?
                .try_join_thread(pid, tid);

            match result {
                Ok(zombie_thread) => {
                    let status: ExitStatus = zombie_thread.status();

                    // Harvest zombie thread.
                    if let (Some(_kernel_stack), Some(user_stack)) = zombie_thread.harvest() {
                        // Traverse pages belonging to user stack.
                        let base: usize = user_stack.base().into_raw_value();
                        let top: usize = user_stack.top().into_raw_value();
                        // TODO: Use an iterator for this.
                        for raw_addr in (base..top).step_by(PAGE_SIZE) {
                            let vaddr: PageAligned<VirtualAddress> =
                                match PageAligned::from_raw_value(raw_addr) {
                                    Ok(vaddr) => vaddr,
                                    Err(_) => {
                                        // SAFETY: the following condition is unreachable, because
                                        // pages in the user stack are always page-aligned.
                                        unreachable!("address conversion should succeed")
                                    },
                                };
                            // Attempt to unmap page
                            if let Err(error) = VirtMemoryManager::get_mut().unmap_upage(
                                Self::get_mut()
                                    .try_borrow_mut()
                                    .map_err(SleepError::Generic)?
                                    .find_process_mut(pid)
                                    .map_err(SleepError::Generic)?
                                    .state_mut()
                                    .vmem_mut(),
                                vaddr,
                            ) {
                                // We failed, but this is not too bad, as we will free all pages
                                // when wiping out the address space anyways.
                                warn!(
                                    "harvest_zombies(): failed to unmap page (vaddr={:?}, \
                                     error={:?})",
                                    vaddr, error
                                );
                            }
                        }

                        // Frames allocated to the user stack are freed when we exit this scope.
                        // Frames allocated to the kernel stack are freed when we exit this scope.
                    }

                    break Ok(status);
                },

                Err(Ok(join_cond)) => {
                    join_cond.wait(None)?;
                },

                Err(Err(error)) => break Err(SleepError::Generic(error)),
            }
        }
    }

    ///
    /// # Description
    ///
    /// Suspends the execution of the calling thread until it is woken up by another thread or until
    /// the specified alarm time is reached.
    ///
    /// # Parameters
    ///
    /// - `alarm`: Optional alarm time.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs a context switch, suspending the execution of
    /// the current thread until it is woken up by another thread or until the specified alarm time
    /// is reached.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The calling thread is not a kernel thread.
    /// - The process manager is initialized.
    /// - The calling thread does not hold a reference to the process manager.
    /// - Access to the process manager is synchronized.
    /// - The processor is running with interrupts disabled.
    /// - The processor is running in privileged mode.
    ///
    pub unsafe fn sleep(alarm: Option<SystemTime>) -> Result<(), SleepError> {
        // Suspend the execution of the calling thread and select another thread to run next.
        let (next_pid, next_tid, from, to, user_tda): (
            ProcessIdentifier,
            ThreadIdentifier,
            *mut ContextInformation,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = Self::get_mut()
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .sleep(alarm);

        // SAFETY: `from` and `to` point to valid context information structures, and the processor
        // is running with interrupts disabled.
        PERF_SCHED_SLEEP_CONTEXT_SWITCHES.fetch_add(1, ORDER);
        Self::switch(next_pid, next_tid, from, to, user_tda);

        // Check the reason why the thread was woken up.
        let interrupt_reason: Option<InterruptReason> = Self::get_mut()
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .interrupt_reason();

        // Check if the thread was interrupted.
        if let Some(reason) = interrupt_reason {
            warn!("interrupted (reason={reason:?})");
            return Err(SleepError::Interrupted(reason));
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Gives up the processor and schedules another ready thread to run.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    /// This function may not return immediately, as the scheduler algorithm may select another
    /// thread to run before of the calling thread.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs a context switch, suspending the execution of
    /// the current thread until it is re-scheduled for execution.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The process manager is initialized.
    /// - The calling thread does not hold a reference to the process manager.
    /// - Access to the process manager is synchronized.
    /// - The processor is running with interrupts disabled.
    /// - The processor is running in privileged mode.
    ///
    pub unsafe fn giveup() -> Result<(), Error> {
        // Check the remaining quantum for the current thread to decide whether to perform a context switch.
        let remaining_ticks: usize = REMAINING_QUANTUM.load(ORDER);
        if remaining_ticks > 1 {
            // The current thread still has remaining quantum, no context switch is required.
            REMAINING_QUANTUM.store(remaining_ticks - 1, ORDER);
        } else {
            // The current thread has no remaining quantum, perform a context switch.

            cold_path();

            // Re-schedule the calling thread and select another thread to run next.
            let (next_pid, next_tid, from, to, user_tda): (
                ProcessIdentifier,
                ThreadIdentifier,
                *mut ContextInformation,
                *mut ContextInformation,
                Option<VirtualAddress>,
            ) = Self::get_mut().try_borrow_mut()?.schedule();

            // Switch to the next thread and updating the remaining quantum accordingly.
            // SAFETY: `from` and `to` point to valid context information structures, and the
            // processor is running with interrupts disabled.
            PERF_SCHED_GIVEUP_CONTEXT_SWITCHES.fetch_add(1, ORDER);
            Self::switch(next_pid, next_tid, from, to, user_tda);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Returns a mutex that is associated with the given address.
    ///
    /// # Parameters
    ///
    /// - `mutex_addr`: Address of the mutex.
    ///
    /// # Returns
    ///
    /// On success, the mutex that is associated with the given address is returned.  If no mutex is
    /// associated with the given address, a new mutex is created and returned. On failure, an
    /// error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn get_mutex(addr: MutexAddress) -> Result<Mutex, Error> {
        Self::get_mut().try_borrow_mut()?.get_mutex(addr)
    }

    ///
    /// # Description
    ///
    /// Stores a mutex guard in the calling thread.
    ///
    /// # Parameters
    ///
    /// - `mutex_addr`: Address of the mutex.
    /// - `guard`: Mutex guard to store.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn put_mutex_guard(
        mutex_addr: MutexAddress,
        guard: MutexGuard,
    ) -> Result<(), Error> {
        Self::get_mut()
            .try_borrow_mut()?
            .put_mutex_guard(mutex_addr, guard);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Returns a condition variable that is associated with the given address.
    ///
    /// # Parameters
    ///
    /// - `cond_addr`: Address of the condition variable.
    ///
    /// # Returns
    ///
    /// On success, the condition variable that is associated with the given address is returned. If
    /// no condition variable is associated with the given address, a new condition variable is
    /// created and returned. On failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn get_cond(cond_addr: ConditionAddress) -> Result<Condvar, Error> {
        Self::get_mut().try_borrow_mut()?.get_cond(cond_addr)
    }

    ///
    /// # Description
    ///
    /// Releases a condition variable associated with the given address.
    ///
    /// # Parameters
    ///
    /// - `cond_addr`: Address of the condition variable.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn put_cond(cond_addr: ConditionAddress) -> Result<(), Error> {
        Self::get_mut().try_borrow_mut()?.put_cond(cond_addr)
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the message is returned. Otherwise, an error code is returned
    /// instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn try_recv(tid: ThreadIdentifier) -> Result<Option<Message>, Error> {
        let mut pm: RefMut<ProcessManagerInner> = unsafe { Self::get_mut() }.try_borrow_mut()?;
        let running: &mut RunningProcess = pm.get_running_mut();
        match running.state_mut().receive_message(tid) {
            Some(message) => {
                pm.number_buffered_messages -= 1;
                Ok(Some(message))
            },
            None => Ok(None),
        }
    }

    ///
    /// # Description
    ///
    /// Wakes up a thread.
    ///
    /// # Parameters
    ///
    /// - `tid`: ID of the thread to wake up.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn wakeup(tid: ThreadIdentifier) -> Result<(), Error> {
        PERF_SCHED_WAKEUP.fetch_add(1, ORDER);
        Self::get_mut().try_borrow_mut()?.wakeup(tid)
    }

    ///
    /// # Description
    ///
    /// Takes a mutex guard from a thread.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `tid`: Thread identifier.
    /// - `mutex_addr`: Address of the mutex.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the mutex guard is returned. Otherwise, an error is returned
    /// instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn take_mutex_guard(
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        mutex_addr: MutexAddress,
    ) -> Result<MutexGuard, Error> {
        Self::get_mut()
            .try_borrow_mut()?
            .take_mutex_guard(pid, tid, mutex_addr)
    }

    ///
    /// # Description
    ///
    /// Checks if the kernel is running.
    ///
    /// # Returns
    ///
    /// Returns true if the kernel is running, false otherwise.
    ///
    pub fn is_kernel_running() -> bool {
        CURRENT_TID.load(ORDER) == ThreadIdentifier::KERNEL_RAW
    }

    ///
    /// # Description
    ///
    /// Switches the execution to another thread.
    ///
    /// # Parameters
    ///
    /// - `next_pid`: Process identifier of the next thread to run.
    /// - `next_tid`: Thread identifier of the next thread to run.
    /// - `from`: Pointer to the context information of the current thread.
    /// - `to`: Pointer to the context information of the next thread.
    /// - `user_tda`: Optional base address for the user-space the thread data area of the next thread to run.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs a context switch between two execution contexts.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - `from` and `to` point to valid execution contexts.
    /// - The processor is running with interrupts disabled.
    /// - The processor is running in privileged mode.
    ///
    /// # Notes
    ///
    /// This function does not return to the caller immediately. Instead, it switches to the `to`
    /// context. When the `from` context is switched back to, this function will return.
    ///
    #[inline(always)]
    unsafe fn switch(
        next_pid: ProcessIdentifier,
        next_tid: ThreadIdentifier,
        from: *mut ContextInformation,
        to: *mut ContextInformation,
        user_tda: Option<VirtualAddress>,
    ) {
        let previous_pid: ProcessIdentifier = ProcessIdentifier::from(CURRENT_PID.load(ORDER));
        let previous_tid: ThreadIdentifier = ThreadIdentifier::from(CURRENT_TID.load(ORDER));

        // Check if we need to perform a context switch.
        if next_tid != previous_tid {
            // We need to perform a context switch.
            PERF_SCHED_HARD_CONTEXT_SWITCHES.fetch_add(1, ORDER);

            // Check whether we need to reset the quantum for the next thread.
            if next_pid != previous_pid {
                REMAINING_QUANTUM.store(SCHEDULER_FREQ, ORDER);
                CURRENT_PID.store(next_pid.into(), ORDER);
            }
            CURRENT_TID.store(next_tid.into(), ORDER);

            ContextInformation::switch(from, to, user_tda);
        } else {
            // We do not need to perform a context switch, the same thread will continue running.
            PERF_SCHED_SOFT_CONTEXT_SWITCHES.fetch_add(1, ORDER);

            // Check if the kernel thread will continue running.
            if next_tid == ThreadIdentifier::KERNEL {
                PERF_SCHED_KERNEL_IDLE.fetch_add(1, ORDER);

                // The kernel thread will continue running. This means there are no other ready
                // threads to run, and the kernel has no work to do at the moment. Enable interrupts
                // and wait for an external event (such as a timer or hardware interrupt) to wake up
                // the kernel.

                // SAFETY: Enabling interrupts in this scope will not cause unwanted side effects.
                let interrupts: Interrupts = Interrupts::enable();

                // SAFETY: Waiting for interrupts will not cause unwanted side effects.
                interrupts.wait();

                // Interrupts are automatically disabled when we leave this scope.
            }
        }
    }
}
