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
            manager::ProcessManager,
            state::{
                ProcessRefMut,
                RunningProcess,
            },
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
use ::alloc::vec::Vec;
use ::arch::mem::PAGE_SIZE;
use ::config::kernel::SCHEDULER_FREQ;
use ::core::{
    hint::{
        cold_path,
        unlikely,
    },
    mem::MaybeUninit,
    sync::atomic::{
        AtomicBool,
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

/// Process manager storage.
static mut PROCESS_MANAGER: MaybeUninit<ProcessManager> = MaybeUninit::uninit();

/// Whether the process manager has been initialized.
static PROCESS_MANAGER_INIT: AtomicBool = AtomicBool::new(false);

/// ID of the current process.
static CURRENT_PID: AtomicI32 = AtomicI32::new(ProcessIdentifier::KERNEL_RAW);

/// ID of the current thread.
pub(super) static CURRENT_TID: AtomicI32 = AtomicI32::new(ProcessIdentifier::KERNEL_RAW);

/// Remaining quantum for the current thread.
static REMAINING_QUANTUM: AtomicUsize = AtomicUsize::new(SCHEDULER_FREQ);

/// ID of thread that owns the FPU.
pub(super) static FPU_OWNER_TID: AtomicI32 = AtomicI32::new(ThreadIdentifier::KERNEL_RAW);

/// Nesting depth of exception handlers currently being served.
static SERVING_EXCEPTION: AtomicUsize = AtomicUsize::new(0);

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
    /// # Panics
    ///
    /// This function panics if the process manager is already initialized.
    ///
    pub fn init(interrupt_capable: bool, kernel: ReadyThread, root: Vmem, tm: ThreadManager) {
        // Check if the process manager is already initialized.
        if unlikely(PROCESS_MANAGER_INIT.load(ORDER)) {
            panic!("process manager was already initialized");
        }

        let pm: ProcessManager = ProcessManager::new(interrupt_capable, kernel, root, tm);

        // SAFETY: This happens during kernel initialization and no other threads are running.
        unsafe { PROCESS_MANAGER.write(pm) };
        PROCESS_MANAGER_INIT.store(true, ORDER);
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
        if unlikely(!PROCESS_MANAGER_INIT.load(ORDER)) {
            panic!("process manager is not initialized");
        }

        // SAFETY: The process manager has been initialized, so the value is valid.
        PROCESS_MANAGER.assume_init_ref()
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
        if unlikely(!PROCESS_MANAGER_INIT.load(ORDER)) {
            panic!("process manager is not initialized");
        }

        // SAFETY: The process manager has been initialized, so the value is valid.
        PROCESS_MANAGER.assume_init_mut()
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

        // Reap any detached-thread zombies deferred from a previous context switch.
        Self::reap_deferred();

        // Terminate the calling process and select another process to run next.
        let (next_pid, next_tid, from, to, user_tda): (
            ProcessIdentifier,
            ThreadIdentifier,
            *mut ContextInformation,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = Self::get_mut().do_exit(status);

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
        // Reap any detached-thread zombies deferred from a previous context switch.
        Self::reap_deferred();

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
            ) = Self::get_mut().do_exit_thread(status);

            join_cond.notify_all()?;

            (next_pid, next_tid, from, to, user_tda)
        };

        // SAFETY: `from` and `to` point to valid context information structures, and the processor
        // is running with interrupts disabled.

        // Debug-only: detect use-after-free in the detached-thread exit path. With slab
        // poison-on-free, a freed ContextInformation block is filled with SLAB_POISON_BYTE. If
        // `from` points to a poisoned block, the zombie thread's ContextInformation was freed
        // before the context switch — a use-after-free bug.
        #[cfg(all(debug_assertions, not(verus_keep_ghost)))]
        {
            let from_bytes: &[u8] = unsafe {
                core::slice::from_raw_parts(
                    from as *const u8,
                    core::mem::size_of::<ContextInformation>(),
                )
            };
            debug_assert!(
                !from_bytes.iter().all(|&b| b == slab::SLAB_POISON_BYTE),
                "BUG: ContextInformation at {:p} freed before context switch (detached-thread UAF)",
                from,
            );
        }

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
    /// Reaps any detached-thread zombies whose cleanup was deferred because
    /// their `ContextInformation` was still needed by an in-progress context
    /// switch. This must be called at PM entry points so that deferred zombies
    /// are cleaned up once the context switch that produced them has completed.
    ///
    /// # Safety
    ///
    /// This function is safe to call if and only if the following conditions are met:
    /// - The process manager is initialized.
    /// - Access to the process manager is synchronized.
    /// - The memory manager is initialized.
    /// - Access to the memory manager is synchronized.
    ///
    unsafe fn reap_deferred() {
        let deferred: Vec<(ProcessIdentifier, ZombieThread)> =
            core::mem::take(&mut Self::get_mut().deferred_reap);
        for (pid, zombie) in deferred {
            Self::harvest_zombie_thread(pid, zombie);
        }
    }

    ///
    /// # Description
    ///
    /// Harvests a zombie thread by reclaiming its kernel and user stacks, unmapping user-stack
    /// pages from the process address space, and notifying the thread manager.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier that owns the zombie thread.
    /// - `zombie_thread`: The zombie thread to harvest.
    ///
    /// # Safety
    ///
    /// This function is safe to call if and only if the following conditions are met:
    /// - The process manager is initialized.
    /// - Access to the process manager is synchronized.
    /// - The memory manager is initialized.
    /// - Access to the memory manager is synchronized.
    ///
    unsafe fn harvest_zombie_thread(pid: ProcessIdentifier, zombie_thread: ZombieThread) {
        // If the zombie has no user stack (kernel-only thread), the kernel stack is
        // reclaimed via KernelStack::Drop and no page unmapping is needed.
        if let (Some(_kernel_stack), Some(user_stack)) = zombie_thread.harvest() {
            // Traverse pages belonging to user stack.
            let base: usize = user_stack.base().into_raw_value();
            let top: usize = user_stack.top().into_raw_value();
            // Resolve the process vmem once before the loop.
            let mut process_ref: ProcessRefMut<'_> = match Self::get_mut().find_process_mut(pid) {
                Ok(process) => process,
                Err(error) => {
                    // Unexpected failure — log but continue since the address space will be
                    // reclaimed when it is destroyed.
                    error!("failed to find process (pid={pid:?}, error={error:?})",);
                    return;
                },
            };
            let vmem: &mut Vmem = process_ref.state_mut().vmem_mut();
            // TODO: Use an iterator for this.
            for raw_addr in (base..top).step_by(PAGE_SIZE) {
                let vaddr: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(raw_addr)
                {
                    Ok(vaddr) => vaddr,
                    Err(_) => {
                        // SAFETY: the following condition is unreachable, because
                        // pages in the user stack are always page-aligned.
                        unreachable!("address conversion should succeed")
                    },
                };
                // Attempt to unmap page.
                match VirtMemoryManager::get_mut().try_unmap_upage(vmem, vaddr) {
                    Ok(true) => {
                        // Page was present and has been successfully unmapped.
                    },
                    Ok(false) => {
                        // Page was never mapped (not demand-paged). Skip silently.
                    },
                    Err(error) => {
                        // Unexpected failure — log but continue since the
                        // address space will be reclaimed when it is destroyed.
                        warn!(
                            "harvest_zombie_thread(): failed to unmap page (vaddr={:?}, \
                             error={:?})",
                            vaddr, error
                        );
                    },
                }
            }

            // Frames allocated to the user stack are freed when we exit this scope.
            // Frames allocated to the kernel stack are freed when we exit this scope.
        }

        // Notify the thread manager that this thread has been reaped.
        Self::get_mut().tm.on_thread_reaped();
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

        // Reap any detached-thread zombies deferred from a previous context switch.
        Self::reap_deferred();

        loop {
            let result: Result<ZombieThread, Result<Condvar, Error>> =
                Self::get_mut().try_join_thread(pid, tid);

            match result {
                Ok(zombie_thread) => {
                    let status: ExitStatus = zombie_thread.status();
                    Self::harvest_zombie_thread(pid, zombie_thread);
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
    /// Detaches a thread in the calling process. A detached thread is automatically harvested when
    /// it exits, without requiring another thread to join it.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the calling process.
    /// - `tid`: Thread identifier of the thread to detach.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
    ///
    /// # Safety
    ///
    /// This function is safe to call if and only if the following conditions are met:
    /// - The process manager is initialized.
    /// - Access to the process manager is synchronized.
    /// - The memory manager is initialized.
    /// - Access to the memory manager is synchronized.
    ///
    pub unsafe fn detach_thread(
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
    ) -> Result<(), Error> {
        trace!("pid={:?}, tid={:?}", pid, tid);

        // Reap any detached-thread zombies deferred from a previous context switch.
        Self::reap_deferred();

        let result: Result<Option<ZombieThread>, Error> =
            Self::get_mut().do_detach_thread(pid, tid);

        match result {
            Ok(Some(zombie_thread)) => {
                // Thread was already a zombie — harvest it immediately.
                Self::harvest_zombie_thread(pid, zombie_thread);
                Ok(())
            },
            Ok(None) => Ok(()),
            Err(error) => Err(error),
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
        // Reap any detached-thread zombies deferred from a previous context switch.
        Self::reap_deferred();

        // Suspend the execution of the calling thread and select another thread to run next.
        let (next_pid, next_tid, from, to, user_tda): (
            ProcessIdentifier,
            ThreadIdentifier,
            *mut ContextInformation,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = Self::get_mut().do_sleep(alarm);

        // SAFETY: `from` and `to` point to valid context information structures, and the processor
        // is running with interrupts disabled.
        PERF_SCHED_SLEEP_CONTEXT_SWITCHES.fetch_add(1, ORDER);
        Self::switch(next_pid, next_tid, from, to, user_tda);

        // Check the reason why the thread was woken up.
        let interrupt_reason: Option<InterruptReason> = Self::get_mut().interrupt_reason();

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
    /// Ticks the scheduler, performing a context switch if the current thread's quantum has
    /// expired.
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
    pub unsafe fn tick() -> Result<(), Error> {
        // Check the remaining quantum for the current thread to decide whether to perform a context switch.
        let remaining_ticks: usize = REMAINING_QUANTUM.load(ORDER);
        if remaining_ticks > 1 {
            // The current thread still has remaining quantum, no context switch is required.
            REMAINING_QUANTUM.store(remaining_ticks - 1, ORDER);
        } else {
            // The current thread has no remaining quantum, perform a context switch.
            cold_path();
            Self::giveup()?
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
    /// thread to run before the calling thread.
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
        // Reap any detached-thread zombies deferred from a previous context switch.
        Self::reap_deferred();

        REMAINING_QUANTUM.store(0, ORDER);

        // Re-schedule the calling thread and select another thread to run next.
        let (next_pid, next_tid, from, to, user_tda): (
            ProcessIdentifier,
            ThreadIdentifier,
            *mut ContextInformation,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = Self::get_mut().schedule();

        // Switch to the next thread and updating the remaining quantum accordingly.
        // SAFETY: `from` and `to` point to valid context information structures, and the
        // processor is running with interrupts disabled.
        PERF_SCHED_GIVEUP_CONTEXT_SWITCHES.fetch_add(1, ORDER);
        Self::switch(next_pid, next_tid, from, to, user_tda);

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
        Self::get_mut().lookup_mutex(addr)
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
        Self::get_mut().store_mutex_guard(mutex_addr, guard);
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
        Self::get_mut().lookup_cond(cond_addr)
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
        Self::get_mut().release_cond(cond_addr)
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
        let pm: &mut ProcessManager = unsafe { Self::get_mut() };
        let running: &mut RunningProcess = pm.get_running_mut();
        match running.state_mut().receive_message(tid) {
            Some(message) => {
                pm.note_message_received()?;
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
        Self::get_mut().do_wakeup(tid)
    }

    ///
    /// # Description
    ///
    /// Best-effort wakeup of a thread used by synchronization primitives.
    ///
    /// # Parameters
    ///
    /// - `tid`: ID of the thread to wake up.
    ///
    /// # Returns
    ///
    /// Returns `true` if a sleeping thread with the given identifier was woken, or `false` if no
    /// such thread is currently sleeping (e.g., it already timed out before this call). Unlike
    /// [`Self::wakeup`], the not-sleeping case is not treated as an error, so a stale waiter neither
    /// consumes a notification nor produces a spurious error log.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn wakeup_waiter(tid: ThreadIdentifier) -> bool {
        PERF_SCHED_WAKEUP.fetch_add(1, ORDER);
        Self::get_mut().try_wakeup_thread(tid)
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
        Self::get_mut().remove_mutex_guard(pid, tid, mutex_addr)
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
    /// Computes the quantum that the next thread to run should start with when the scheduler
    /// switches away from a thread of process `previous_pid` to a thread of process `next_pid`.
    ///
    /// # Parameters
    ///
    /// - `next_pid`: Process identifier of the next thread to run.
    /// - `previous_pid`: Process identifier of the thread that is being switched out.
    /// - `remaining`: Quantum that remained for the thread that is being switched out.
    ///
    /// # Returns
    ///
    /// The quantum that the next thread should start running with.
    ///
    pub(super) fn next_thread_quantum(
        next_pid: ProcessIdentifier,
        previous_pid: ProcessIdentifier,
        remaining: usize,
    ) -> usize {
        // Reset the quantum to a full slice on a cross-process switch, and also on an intra-process
        // switch when the outgoing thread had already exhausted its quantum (i.e. it was preempted
        // through tick()/giveup()). The latter prevents the incoming thread from inheriting an
        // exhausted quantum and being immediately preempted on the next tick, which would otherwise
        // starve it relative to its sibling threads.
        //
        // Otherwise, on an intra-process switch where the outgoing thread still had quantum to
        // spare (e.g. a voluntary yield through sleep()/exit()), the incoming thread inherits the
        // remaining quantum so that a process cannot accumulate more than its fair share of CPU
        // time across its threads.
        if next_pid != previous_pid || remaining == 0 {
            SCHEDULER_FREQ
        } else {
            remaining
        }
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

            // Reset the quantum for the next thread, as required.
            let remaining: usize = REMAINING_QUANTUM.load(ORDER);
            REMAINING_QUANTUM
                .store(Self::next_thread_quantum(next_pid, previous_pid, remaining), ORDER);
            CURRENT_PID.store(next_pid.into(), ORDER);
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

    ///
    /// # Description
    ///
    /// Marks the CPU as serving an exception and returns an RAII guard that decrements the nesting
    /// depth on drop. Supports nesting: each call increments a depth counter and the corresponding
    /// guard decrements it, so the flag remains set until all guards have been dropped.
    ///
    /// # Returns
    ///
    /// An [`ExceptionGuard`] that decrements the nesting depth when dropped.
    ///
    pub fn enter_exception_handler() -> ExceptionGuard {
        SERVING_EXCEPTION.fetch_add(1, ORDER);
        ExceptionGuard(())
    }

    ///
    /// # Description
    ///
    /// Returns whether the CPU is currently serving an exception.
    ///
    /// # Returns
    ///
    /// `true` if the exception handler flag is set, `false` otherwise.
    ///
    pub(super) fn is_serving_exception() -> bool {
        SERVING_EXCEPTION.load(ORDER) > 0
    }
}

/// RAII guard that decrements the `SERVING_EXCEPTION` nesting depth on drop.
///
/// Cannot be constructed outside this module — only [`ProcessManager::enter_exception_handler`]
/// produces instances.
#[must_use = "guard must be held for the duration of the exception handler"]
pub struct ExceptionGuard(());

impl Drop for ExceptionGuard {
    fn drop(&mut self) {
        SERVING_EXCEPTION.fetch_sub(1, ORDER);
    }
}
