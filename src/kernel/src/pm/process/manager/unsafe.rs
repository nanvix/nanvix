// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Global Variables
//==================================================================================================

static mut PROCESS_MANAGER: Option<ProcessManager> = None;

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
    },
};
use ::alloc::rc::Rc;
use ::arch::mem::PAGE_SIZE;
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
        Ordering,
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

/// PID of the current process.
static CURRENT_PID: AtomicI32 = AtomicI32::new(ProcessIdentifier::KERNEL_RAW);

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
    /// Exits the calling process.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function does not return. Otherwise, an error code is
    /// returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it may cause the calling process to exit.
    ///
    /// This function is safe to use if an only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    ///
    pub unsafe fn exit(status: ExitStatus) -> Result<!, Error> {
        trace!("exit(): status={:?}", status);
        // SAFETY: This is the only thread running, thus access to the process manager is synchronized.
        let (from, to): (*mut ContextInformation, *mut ContextInformation) =
            unsafe { Self::get_mut() }.try_borrow_mut()?.exit(status);

        ContextInformation::switch(from, to);
        core::hint::unreachable_unchecked()
    }

    ///
    /// # Description
    ///
    /// Exits the calling thread.
    ///
    /// # Parameters
    ///
    /// - `status`: Exit status.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function does not return. Otherwise, an error code is
    /// returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it operates on global variables and it may panic.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    /// - The calling process does not hold a reference to the process manager.
    ///
    pub unsafe fn exit_thread(status: ExitStatus) -> Result<!, Error> {
        let (from, to): (*mut ContextInformation, *mut ContextInformation) = {
            // Create a scope so the join condition variable is dropped before we context switch.
            // If we do not do this, the condition variable the reference count for the condition
            // variable will not be decremented, causing a memory leak.

            let (join_cond, from, to): (Condvar, *mut ContextInformation, *mut ContextInformation) =
                Self::get_mut().try_borrow_mut()?.exit_thread(status);

            join_cond.notify_all()?;

            (from, to)
        };

        ContextInformation::switch(from, to);
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
        trace!("join_thread(): pid={:?}, tid={:?}", pid, tid);

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
    /// Puts the calling thread to sleep.
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
    /// This function is unsafe because it performs a context switch, causing the current thread to
    /// block until it is woken up by another thread.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    ///
    pub unsafe fn sleep(alarm: Option<SystemTime>) -> Result<(), SleepError> {
        let (from, to): (*mut ContextInformation, *mut ContextInformation) = Self::get_mut()
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .sleep(alarm);

        ContextInformation::switch(from, to);

        let interrupt_reason: Option<InterruptReason> = Self::get_mut()
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .interrupt_reason();

        if let Some(reason) = interrupt_reason {
            error!("sleep(): interrupted (reason={:?})", reason);
            return Err(SleepError::Interrupted(reason));
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Switches the context of the calling thread with the next ready thread.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs a context switch, causing the current thread to
    /// block until it is woken up by another thread.
    ///
    pub unsafe fn switch() -> Result<(), Error> {
        // Schedule the next thread within a limited scope to ensure the process manager is
        // properly released before performing the context switch.
        let result: Option<(ProcessIdentifier, *mut ContextInformation, *mut ContextInformation)> =
            { Self::get_mut().try_borrow_mut()?.schedule() };

        match result {
            Some((next_pid, from, to)) => {
                // SAFETY: `from` and `to` point to valid context information structures. The
                // processor is running with interrupts disabled.
                CURRENT_PID.store(next_pid.into(), Ordering::SeqCst);
                ContextInformation::switch(from, to);
            },
            None => {
                // SAFETY: Enabling interrupts in this scope will not cause unwanted side effects.
                let interrupts: Interrupts = Interrupts::enable();

                // SAFETY: Waiting for interrupts will not cause unwanted side effects.
                interrupts.wait();

                // Interrupts are automatically disabled when we leave this scope.
            },
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
    /// # Safety
    ///
    /// This function is unsafe because it access global variables.
    ///
    pub unsafe fn is_kernel_running() -> bool {
        CURRENT_PID.load(Ordering::SeqCst) == ProcessIdentifier::KERNEL_RAW
    }
}
