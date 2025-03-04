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
use ::alloc::{
    rc::Rc,
    sync::Arc,
};
use ::core::{
    cell::{
        RefCell,
        RefMut,
    },
    hint::{
        cold_path,
        unlikely,
    },
};
use ::sys::{
    arch::mem::PAGE_SIZE,
    error::Error,
    ipc::Message,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

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
    pub unsafe fn exit(status: i32) -> Result<!, Error> {
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
    pub unsafe fn exit_thread(status: usize) -> Result<!, Error> {
        let (join_cond, from, to): (
            Arc<Condvar>,
            *mut ContextInformation,
            *mut ContextInformation,
        ) = Self::get_mut().try_borrow_mut()?.exit_thread(status);

        join_cond.notify_all()?;

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
    ) -> Result<usize, SleepError> {
        trace!("join_thread(): pid={:?}, tid={:?}", pid, tid);

        loop {
            let result: Result<ZombieThread, Result<Arc<Condvar>, Error>> = Self::get_mut()
                .try_borrow_mut()
                .map_err(SleepError::Generic)?
                .try_join_thread(pid, tid);

            match result {
                Ok(zombie_thread) => {
                    let status: usize = zombie_thread.status();

                    // Harvest zombie thread.
                    if let Some(user_stack) = zombie_thread.harvest() {
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
                    }

                    break Ok(status);
                },

                Err(Ok(join_cond)) => {
                    join_cond.wait()?;
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
    pub unsafe fn sleep() -> Result<(), SleepError> {
        let (from, to): (*mut ContextInformation, *mut ContextInformation) = Self::get_mut()
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .sleep();

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
        let (from, to): (*mut ContextInformation, *mut ContextInformation) =
            { Self::get_mut().try_borrow_mut()?.schedule() };

        ContextInformation::switch(from, to);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Returns a mutex that is associated with the given address.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address of the mutex.
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
    pub unsafe fn get_mutex(addr: VirtualAddress) -> Result<Mutex, Error> {
        Self::get_mut().try_borrow_mut()?.get_mutex(addr)
    }

    ///
    /// # Description
    ///
    /// Stores a mutex guard in the calling thread.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address of the mutex.
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
    pub unsafe fn put_mutex_guard(addr: VirtualAddress, guard: MutexGuard) -> Result<(), Error> {
        Self::get_mut()
            .try_borrow_mut()?
            .put_mutex_guard(addr, guard);
        Ok(())
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
    pub unsafe fn try_recv() -> Result<Option<Message>, Error> {
        let mut pm: RefMut<ProcessManagerInner> = unsafe { Self::get_mut() }.try_borrow_mut()?;
        let running: &mut RunningProcess = pm.get_running_mut();
        match running.state_mut().receive_message() {
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
    /// - `pid`: ID of the process to wake up.
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
    pub unsafe fn wakeup(pid: ProcessIdentifier, tid: ThreadIdentifier) -> Result<(), Error> {
        Self::get_mut().try_borrow_mut()?.wakeup(pid, tid)
    }
}
