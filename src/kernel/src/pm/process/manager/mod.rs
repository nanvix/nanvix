// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

mod r#unsafe;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventOwnership,
    hal::{
        self,
        arch::ContextInformation,
        io::{
            AnyIoPort,
            IoMemoryRegion,
            IoPortWidth,
        },
        mem::{
            AccessPermission,
            Address,
            PageAligned,
            VirtualAddress,
        },
    },
    mm::{
        elf::Elf32Fhdr,
        kstack::KernelStack,
        ustack::{
            UserStack,
            UserStackAllocator,
        },
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        clock,
        process::state::{
            InterruptedProcess,
            ProcessRef,
            ProcessRefMut,
            ProcessState,
            RunnableProcess,
            RunningProcess,
            SleepingProcess,
            ZombieProcess,
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
    },
};
use ::alloc::{
    boxed::Box,
    collections::{
        vec_deque::VecDeque,
        LinkedList,
    },
    ffi::CString,
    rc::Rc,
};
use ::arch::mem::PAGE_SIZE;
use ::core::cell::{
    Ref,
    RefCell,
    RefMut,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    event::Event,
    ipc::Message,
    pm::{
        Capability,
        ConditionAddress,
        MutexAddress,
        ProcessIdentifier,
        ThreadIdentifier,
    },
    time::SystemTime,
    ExitStatus,
};
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Sleep Error
//==================================================================================================

#[derive(Debug)]
pub enum SleepError {
    Interrupted(InterruptReason),
    Generic(Error),
}

//==================================================================================================
// Process Manager Inner
//==================================================================================================

///
/// # Description
///
/// A type that represents the process manager.
///
struct ProcessManagerInner {
    /// Is this platform interrupt capable?
    interrupt_capable: bool,
    /// Reason for the last interrupt.
    interrupt_reason: Option<InterruptReason>,
    /// Next process identifier.
    next_pid: ProcessIdentifier,
    /// Running process.
    running: Option<RunningProcess>,
    /// Ready processes.
    ready: LinkedList<RunnableProcess>,
    /// Suspended processes.
    suspended: LinkedList<SleepingProcess>,
    /// Interrupted processes.
    interrupted: LinkedList<InterruptedProcess>,
    /// Zombie processes.
    zombies: LinkedList<ZombieProcess>,
    /// Thread manager.
    tm: ThreadManager,
    /// Number of messages buffered (not yet consumed).
    number_buffered_messages: usize,
}

impl ProcessManagerInner {
    /// Initializes the process manager.
    pub fn new(
        interrupt_capable: bool,
        kernel: ReadyThread,
        root: Vmem,
        tm: ThreadManager,
    ) -> Self {
        let kernel: RunnableProcess =
            RunnableProcess::new(ProcessIdentifier::KERNEL, kernel, root, None);

        let (kernel, reason, _): (
            RunningProcess,
            Option<InterruptReason>,
            *mut ContextInformation,
        ) = kernel.run();
        debug_assert!(reason.is_none(), "kernel process should not be interrupted");

        Self {
            interrupt_capable,
            interrupt_reason: None,
            next_pid: ProcessIdentifier::from(1),
            ready: LinkedList::new(),
            suspended: LinkedList::new(),
            interrupted: LinkedList::new(),
            zombies: LinkedList::new(),
            running: Some(kernel),
            tm,
            number_buffered_messages: 0,
        }
    }

    fn forge_user_context(
        mm: &mut VirtMemoryManager,
        vmem: &mut Vmem,
        user_stack: &UserStack,
        user_fn: VirtualAddress,
        arg0: usize,
        arg1: usize,
        enable_interrupts: bool,
    ) -> Result<(KernelStack, ContextInformation), Error> {
        trace!(
            "forge_user_context(): user_stack={:?}, user_wrapper_fn={:#x?}, arg0={:#x?}, \
             arg1={:#x?}, enable_interrupts={:?}",
            user_stack,
            user_fn,
            arg0,
            arg1,
            enable_interrupts
        );

        unsafe extern "C" {
            pub fn __leave_kernel_to_user_mode();
        }

        // Ensure that user wrapper function lies within the user address space.
        if !Vmem::is_user_addr(user_fn) {
            let reason: &str = "user wrapper function is not within the user address space";
            error!(
                "forge_context(): {} (user_stack={:?}, user_func={:?})",
                reason, user_stack, arg0
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // NOTE: we don't check if the user function lies within the user address space, because
        // if it is not it will self-crash.

        let kernel_func: VirtualAddress =
            VirtualAddress::from_raw_value(__leave_kernel_to_user_mode as usize);

        // Alloc kernel pages for the kernel stack.
        let kernel_stack: KernelStack = KernelStack::new(mm)?;

        let cr3: u32 = vmem.pgdir().physical_address()?.into_raw_value() as u32;
        let esp: u32 = unsafe {
            hal::arch::forge_user_stack(
                kernel_stack.top().into_raw_value() as *mut u8,
                user_stack.top().into_raw_value(),
                user_fn.into_raw_value(),
                arg0,
                arg1,
                kernel_func.into_raw_value(),
                enable_interrupts,
            )
        } as u32;
        let esp0: u32 = kernel_stack.top().into_raw_value() as u32;

        trace!("forge_context(): cr3={:#x}, esp={:#x}, ebp={:#x}", cr3, esp, esp0);
        let context: ContextInformation = ContextInformation::new(cr3, esp, esp0);

        // Alloc user stack and map it.
        mm.alloc_upages(
            vmem,
            user_stack.base(),
            user_stack.size() / PAGE_SIZE,
            AccessPermission::RDWR,
        )?;

        // NOTE: if we fail, beyond this point we must unmap kernel pages from `vmem`.

        Ok((kernel_stack, context))
    }

    ///
    /// # Description
    ///
    /// Creates a new thread in the running process.
    ///
    /// # Parameters
    ///
    /// - `mm`: Memory manager to use.
    /// - `pid`: Process identifier.
    /// - `user_func`: User function to execute.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the thread identifier of the new thread is returned.
    /// Otherwise, an error is returned instead.
    ///
    fn create_thread(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        user_wrapper_fn: VirtualAddress,
        user_fn: VirtualAddress,
        user_fn_arg: usize,
    ) -> Result<ThreadIdentifier, Error> {
        trace!(
            "create_thread(): pid={:?}, user_wrapper_fn={:#x?}, user_fn={:#x?}, user_fn_arg={:#x?}",
            pid,
            user_wrapper_fn,
            user_fn,
            user_fn_arg
        );

        let ready_thread: ReadyThread = {
            let enable_interrupts: bool = self.interrupt_capable;

            // Find corresponding process.
            let mut process: ProcessRefMut = self.find_process_mut(pid)?;

            // Ensure that the process is in a valid state.
            if let ProcessRefMut::Running(_) = process {
                // TODO: Re-evaluate this condition when we support multicore.
                let reason: &str = "process is running";
                error!("create_thread(): {}", reason);
                return Err(Error::new(ErrorCode::OperationNotPermitted, reason));
            }
            if let ProcessRefMut::Interrupted(_) = process {
                let reason: &str = "process is interrupted";
                error!("create_thread(): {}", reason);
                return Err(Error::new(ErrorCode::OperationNotPermitted, reason));
            }
            if let ProcessRefMut::Zombie(_) = process {
                let reason: &str = "process is a zombie";
                error!("create_thread(): {}", reason);
                return Err(Error::new(ErrorCode::OperationNotPermitted, reason));
            }
            // TODO: include runnable process with interrupted threads.

            // Allocate a new user stack.
            let user_stack: UserStack = match process.state_mut().get_user_stack_allocator_mut() {
                Some(user_stack_allocator) => user_stack_allocator.alloc()?,
                None => {
                    // The user stack allocator is not available.
                    panic!(
                        "create_thread(): user stack allocator not found, is this the kernel \
                         process?"
                    );
                },
            };

            // Create a kernel context.
            let (kernel_stack, context): (KernelStack, ContextInformation) =
                Self::forge_user_context(
                    mm,
                    process.state_mut().vmem_mut(),
                    &user_stack,
                    user_wrapper_fn,
                    user_fn.into_raw_value(),
                    user_fn_arg,
                    enable_interrupts,
                )?;

            //==============================================================
            // NOTE: if we fail beyond this point we need to page mappings.
            //==============================================================

            // Create a new thread.
            self.tm
                .create_thread(Some(kernel_stack), Some(user_stack), context)
        };

        Ok(self.try_add_thread(pid, ready_thread))
    }

    fn try_add_thread(
        &mut self,
        pid: ProcessIdentifier,
        ready_thread: ReadyThread,
    ) -> ThreadIdentifier {
        trace!("try_add_thread(): pid={pid:?}, ready_thread={ready_thread:?}");
        let tid: ThreadIdentifier = ready_thread.tid();

        // Search process in the list of sleeping processes.
        let mut suspended: LinkedList<SleepingProcess> = LinkedList::new();
        while let Some(process) = self.suspended.pop_front() {
            // Found.
            if process.state().pid() == pid {
                let ready_process: RunnableProcess = process.add_thread(ready_thread);
                // Rollback list to its original state.
                while let Some(process) = suspended.pop_back() {
                    self.suspended.push_front(process);
                }
                // Push process to the list of ready processes.
                self.ready.push_back(ready_process);
                return tid;
            }
            suspended.push_back(process);
        }
        // Process is not in the list of sleeping processes, rollback list to its original state.
        self.suspended = suspended;

        // Search process in the list of ready processes.
        let mut ready: LinkedList<RunnableProcess> = LinkedList::new();
        while let Some(process) = self.ready.pop_front() {
            // Found.
            if process.state().pid() == pid {
                let ready_process: RunnableProcess = process.add_thread(ready_thread);
                // Rollback list to its original state.
                while let Some(process) = ready.pop_back() {
                    self.ready.push_front(process);
                }
                // Push process to the list of ready processes.
                self.ready.push_back(ready_process);
                return tid;
            }
            ready.push_back(process);
        }
        // Process is not in the list of ready processes, rollback list to its original state.
        self.ready = ready;

        unreachable!("process must be either sleeping or runnable")
    }

    ///
    /// # Description
    ///
    /// Creates a new process.
    ///
    /// # Parameters
    ///
    /// - `mm`: Memory manager to use.
    /// - `elf`: ELF header of the executable file.
    /// - `args`: Command line arguments.
    /// - `env`: Environment variables.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the process identifier of the new process is returned.
    /// Otherwise, an error is returned instead.
    ///
    fn create_process(
        &mut self,
        mm: &mut VirtMemoryManager,
        elf: &Elf32Fhdr,
        args: &str,
        env: &str,
    ) -> Result<ProcessIdentifier, Error> {
        unsafe extern "C" {
            pub fn __leave_kernel_to_user_mode();
        }

        trace!("create_process(): args={:?}, env={:?}", args, env);

        // Strip leading and trailing spaces from arguments.
        let args: &str = args.trim();

        // Convert args to C-style string.
        let args: CString = match CString::new(args) {
            Ok(cmdline) => cmdline,
            Err(error) => {
                let reason: &str = "failed to convert command line string";
                error!("create_process(): {} (error={:?})", reason, error);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let args: &[u8] = args.as_bytes_with_nul();

        // Strip leading and trailing spaces from environment variables.
        let env: &str = env.trim();

        // Convert env to C-style string.
        let env: CString = match CString::new(env) {
            Ok(cmdline) => cmdline,
            Err(error) => {
                let reason: &str = "failed to convert environment string";
                error!("create_process(): {} (error={:?})", reason, error);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };
        let env: &[u8] = env.as_bytes_with_nul();

        // Create a new memory address space for the process.
        let mut vmem: Vmem = mm.new_vmem(self.get_running().state().vmem())?;

        // Load the ELF file into the new address space.
        let (entry, args_vaddr): (VirtualAddress, PageAligned<VirtualAddress>) =
            mm.load_elf(&mut vmem, elf)?;

        // Allocate a user-space page, write command line arguments to it, and check for errors.
        // Note we subtract a pointer size from PAGE_SIZE to account for the null terminator.
        if args.len() > PAGE_SIZE - ::core::mem::size_of::<*const u8>() {
            let reason: &str = "command line is too long";
            error!("create_process(): {} (cmdline.len={:?})", reason, args.len());
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        mm.alloc_upage(&mut vmem, args_vaddr, AccessPermission::RDWR, true)?;
        vmem.copy_to_user_unaligned(
            args_vaddr.into_inner(),
            VirtualAddress::new(args.as_ptr() as usize),
            args.len(),
        )?;
        debug!(
            "create_process(): arguments written to user space (args_vaddr={:?}, args={:?})",
            args_vaddr, args
        );

        // Allocate another page for the environment variables and check for errors.
        // Note we subtract a pointer size from PAGE_SIZE to account for the null terminator.
        if env.len() > PAGE_SIZE - ::core::mem::size_of::<*const u8>() {
            let reason: &str = "environment variables are too long";
            error!("create_process(): {} (env.len={:?})", reason, env.len());
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let envp_vaddr: PageAligned<VirtualAddress> = PageAligned::<VirtualAddress>::from_address(
            VirtualAddress::new(args_vaddr.into_raw_value() + PAGE_SIZE),
        )?;
        mm.alloc_upage(&mut vmem, envp_vaddr, AccessPermission::RDWR, true)?;

        // Populate the environment variable page.
        vmem.copy_to_user_unaligned(
            envp_vaddr.into_inner(),
            VirtualAddress::new(env.as_ptr() as usize),
            env.len(),
        )?;
        debug!(
            "create_process(): environment variables written to user space (envp_vaddr={:?}, \
             env={:?})",
            envp_vaddr, env
        );

        // Create a stack allocator.
        let user_stack_allocator: UserStackAllocator = UserStackAllocator::new()?;

        // Create a kernel context.
        let user_stack: UserStack = user_stack_allocator.alloc()?;
        let user_fn: VirtualAddress = entry;
        let argp: usize = args_vaddr.into_raw_value();
        let envp: usize = envp_vaddr.into_raw_value();
        let (kernel_stack, context): (KernelStack, ContextInformation) = Self::forge_user_context(
            mm,
            &mut vmem,
            &user_stack,
            user_fn,
            argp,
            envp,
            self.interrupt_capable,
        )?;

        //==============================================================
        // NOTE: if we fail beyond this point we need to page mappings.
        //==============================================================

        let thread: ReadyThread =
            self.tm
                .create_thread(Some(kernel_stack), Some(user_stack), context);

        // Create process.
        let pid: ProcessIdentifier = self.next_pid;
        self.next_pid = ProcessIdentifier::from(Into::<u32>::into(pid) + 1);
        let process: RunnableProcess =
            RunnableProcess::new(pid, thread, vmem, Some(user_stack_allocator));

        // Add process to the queue of ready processes.
        self.ready.push_back(process);

        Ok(pid)
    }

    /// Schedule a process to run.
    fn schedule(
        &mut self,
    ) -> Option<(ProcessIdentifier, *mut ContextInformation, *mut ContextInformation)> {
        // Reschedule running process.
        let previous_process: RunningProcess = self.take_running();

        let previous_pid: ProcessIdentifier = previous_process.state().pid();

        let (previous_process, previous_context) = previous_process.schedule();
        self.ready.push_back(previous_process);

        self.check_alarm();

        // Select next ready process to run.
        if let Some(next_process) = self.interrupted.pop_front() {
            let (next_process, reason, next_context): (
                RunningProcess,
                InterruptReason,
                *mut ContextInformation,
            ) = next_process.resume();

            let next_pid: ProcessIdentifier = next_process.state().pid();
            self.interrupt_reason = Some(reason);
            self.running = Some(next_process);
            Some((next_pid, previous_context, next_context))
        } else {
            let next_process: RunnableProcess = self.take_ready();
            let (next_process, reason, next_context): (
                RunningProcess,
                Option<InterruptReason>,
                *mut ContextInformation,
            ) = next_process.run();

            self.interrupt_reason = reason;
            let next_pid: ProcessIdentifier = next_process.state().pid();
            self.running = Some(next_process);

            if previous_pid == next_pid {
                if previous_pid != ProcessIdentifier::KERNEL {
                    panic!("schedule(): rescheduling non kernel thread (pid={previous_pid:?})");
                }
                return None;
            }

            Some((next_pid, previous_context, next_context))
        }
    }

    // Traverses list of sleeping processes checking for alarms.
    fn check_alarm(&mut self) {
        let now: SystemTime = clock::now();

        // Create a temporary list to store processes that are still sleeping.
        let mut suspended: LinkedList<SleepingProcess> = LinkedList::new();

        // Filter out processes that are still sleeping.
        while let Some(process) = self.suspended.pop_front() {
            // Attempt to wake up process.
            match process.wakeup_alarm(now) {
                Ok(interrupted_process) => {
                    trace!(
                        "check_alarm(): process {:?} interrupted at {now:?}",
                        interrupted_process.state().pid(),
                    );
                    self.interrupted.push_back(interrupted_process);
                },
                Err(suspended_process) => suspended.push_back(suspended_process),
            }
        }

        // Set the list of sleeping processes.
        self.suspended = suspended;
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
    fn sleep(
        &mut self,
        alarm: Option<SystemTime>,
    ) -> (*mut ContextInformation, *mut ContextInformation) {
        let running_process: RunningProcess = self.take_running();

        // Check if kernel is trying to sleep.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot sleep");
        }

        match running_process.sleep(alarm) {
            Ok((runnable_process, previous_context)) => {
                let (next_process, reason, next_context): (
                    RunningProcess,
                    Option<InterruptReason>,
                    *mut ContextInformation,
                ) = runnable_process.run();
                self.interrupt_reason = reason;
                self.running = Some(next_process);
                (previous_context, next_context)
            },
            Err((suspended_process, previous_context)) => {
                self.suspended.push_back(suspended_process);
                let next_process: RunnableProcess = self.take_ready();
                let (next_process, reason, next_context): (
                    RunningProcess,
                    Option<InterruptReason>,
                    *mut ContextInformation,
                ) = next_process.run();
                self.interrupt_reason = reason;
                self.running = Some(next_process);
                (previous_context, next_context)
            },
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
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    pub fn wakeup(&mut self, pid: ProcessIdentifier, tid: ThreadIdentifier) -> Result<(), Error> {
        // Check if thread belongs to the running process.
        if self.get_running().state().pid() == pid {
            let running_process: RunningProcess = self.take_running();
            match running_process.wakeup(tid) {
                Ok(running_process) => {
                    self.running = Some(running_process);
                    return Ok(());
                },
                Err(running_process) => {
                    self.running = Some(running_process);
                    let reason: &str = "thread not found";
                    error!("wake_up(): {reason} (pid={pid:?}. tid={tid:?})");
                },
            }
        }

        // Check if thread belongs to a suspended process.
        let runnable_process: RunnableProcess = match self.try_wakeup(pid, tid) {
            Some(runnable_process) => runnable_process,
            None => {
                let reason: &str = "thread not found";
                error!("wake_up(): {reason} (pid={pid:?}, tid={tid:?})");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            },
        };

        self.ready.push_back(runnable_process);

        Ok(())
    }

    fn try_wakeup(
        &mut self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
    ) -> Option<RunnableProcess> {
        // Search for the process in the list of sleeping processes.
        let mut suspended: LinkedList<SleepingProcess> = LinkedList::new();
        while let Some(process) = self.suspended.pop_front() {
            // Found.
            if process.state().pid() == pid {
                match process.wakeup(tid) {
                    Ok(runnable_process) => {
                        while let Some(process) = suspended.pop_back() {
                            self.suspended.push_front(process);
                        }
                        return Some(runnable_process);
                    },
                    Err(suspended_process) => suspended.push_back(suspended_process),
                }
            } else {
                suspended.push_back(process)
            }
        }
        // Process is not in the list of sleeping processes, rollback list to its original state.
        self.suspended = suspended;

        // Search for the process in the list of ready processes.
        let mut ready: LinkedList<RunnableProcess> = LinkedList::new();
        while let Some(process) = self.ready.pop_front() {
            // Found.
            if process.state().pid() == pid {
                match process.wakeup(tid) {
                    Ok(runnable_process) => {
                        while let Some(process) = ready.pop_back() {
                            self.ready.push_front(process);
                        }
                        return Some(runnable_process);
                    },
                    Err(ready_process) => ready.push_back(ready_process),
                }
            } else {
                ready.push_back(process)
            }
        }
        // Process is not in the list of ready processes, rollback list to its original state.
        self.ready = ready;

        None
    }

    pub fn exit(
        &mut self,
        status: ExitStatus,
    ) -> (*mut ContextInformation, *mut ContextInformation) {
        let running_process: RunningProcess = self.take_running();

        // Check if kernel is trying to exit.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot exit");
        }

        match running_process.exit(status) {
            Ok((runnable_process, previous_context)) => {
                let (running_process, reason, next_context) = runnable_process.run();
                self.interrupt_reason = reason;
                self.running = Some(running_process);
                (previous_context, next_context)
            },
            Err((zombie_process, previous_context)) => {
                self.zombies.push_back(zombie_process);

                match self.ready.pop_front() {
                    Some(runnable_process) => {
                        let (running_process, reason, next_context) = runnable_process.run();
                        self.interrupt_reason = reason;
                        self.running = Some(running_process);
                        (previous_context, next_context)
                    },
                    None => unreachable!("the kernel process is always ready to run"),
                }
            },
        }
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
    /// This function is unsafe because it may panic.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    ///
    pub(super) unsafe fn exit_thread(
        &mut self,
        status: ExitStatus,
    ) -> (Condvar, *mut ContextInformation, *mut ContextInformation) {
        let running_process: RunningProcess = self.take_running();

        trace!(
            "exit_thread(): pid={:?}, tid={:?}, status={:?}",
            running_process.state().pid(),
            running_process.get_tid(),
            status
        );

        // Check if kernel is trying to exit.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot exit");
        }

        match running_process.exit_thread(status) {
            Ok((join_cond, runnable_process, previous_context)) => {
                let (running_process, reason, next_context) = runnable_process.run();
                self.interrupt_reason = reason;
                self.running = Some(running_process);
                (join_cond, previous_context, next_context)
            },
            Err(Ok((join_cond, sleeping_process, previous_context))) => {
                self.suspended.push_back(sleeping_process);

                match self.ready.pop_front() {
                    Some(runnable_process) => {
                        let (running_process, reason, next_context) = runnable_process.run();
                        self.interrupt_reason = reason;
                        self.running = Some(running_process);
                        (join_cond, previous_context, next_context)
                    },
                    None => unreachable!("the kernel process is always ready to run"),
                }
            },
            Err(Err((join_cond, zombie_process, previous_context))) => {
                self.zombies.push_back(zombie_process);

                match self.ready.pop_front() {
                    Some(runnable_process) => {
                        let (running_process, reason, next_context) = runnable_process.run();
                        self.interrupt_reason = reason;
                        self.running = Some(running_process);
                        (join_cond, previous_context, next_context)
                    },
                    None => unreachable!("the kernel process is always ready to run"),
                }
            },
        }
    }

    pub fn terminate(&mut self, pid: ProcessIdentifier) -> Result<(), Error> {
        // Check if terminating kernel process.
        if pid == ProcessIdentifier::KERNEL {
            let reason: &str = "cannot terminate kernel process";
            error!("terminate(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if target process is running.
        if self.running.is_some() && self.get_running().state().pid() == pid {
            let reason: &str = "cannot terminate running process";
            error!("terminate(): {}", reason);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if target process is ready.
        if let Some(process) = self.ready.iter().position(|p| p.state().pid() == pid) {
            let process: RunnableProcess = self.ready.remove(process);
            match process.terminate() {
                Ok(runnable_process) => {
                    self.ready.push_back(runnable_process);
                    return Ok(());
                },
                Err(zombie_process) => {
                    self.zombies.push_back(zombie_process);
                    return Ok(());
                },
            }
        }

        // Check if target process is suspended.
        if let Some(process) = self.suspended.iter().position(|p| p.state().pid() == pid) {
            let process: SleepingProcess = self.suspended.remove(process);
            let process: InterruptedProcess = process.terminate();
            self.interrupted.push_back(process);
            return Ok(());
        }

        let reason: &str = "process not found";
        error!("terminate(): {}", reason);
        Err(Error::new(ErrorCode::NoSuchProcess, reason))
    }

    ///
    /// # Description
    ///
    /// Sets/clears the capability of a process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `capability`: Capability to set/clear.
    /// - `value`: Set capability if true, clear capability if false.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    pub fn capctl(
        &mut self,
        pid: ProcessIdentifier,
        capability: Capability,
        set: bool,
    ) -> Result<(), Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;

        // Check wether the capability should be set or cleared.
        if set {
            // Check if capability is already set.
            if process.state_mut().has_capability(capability) {
                let reason: &str = "capability already set";
                error!("capctl(): {}", reason);
                return Err(Error::new(ErrorCode::ResourceBusy, reason));
            }
            process.state_mut().set_capability(capability);
        } else {
            // Check if capability is not set.
            if !process.state_mut().has_capability(capability) {
                let reason: &str = "capability not set";
                error!("capctl(): {}", reason);
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            }
            process.state_mut().clear_capability(capability);
        }

        Ok(())
    }

    fn interrupt_reason(&mut self) -> Option<InterruptReason> {
        self.interrupt_reason.take()
    }

    fn harvest_zombies(
        &mut self,
    ) -> Option<(VecDeque<ZombieThread>, Box<ProcessState>, ExitStatus)> {
        if let Some(zombie) = self.zombies.pop_front() {
            let (zombie_threads, state, status): (
                NonEmptyVecDeque<ZombieThread>,
                Box<ProcessState>,
                ExitStatus,
            ) = zombie.bury();
            let (mut more_zombie_threads, zombie_thread): (VecDeque<ZombieThread>, ZombieThread) =
                zombie_threads.pop_front();
            more_zombie_threads.push_front(zombie_thread);

            Some((more_zombie_threads, state, status))
        } else {
            None
        }
    }

    #[allow(clippy::type_complexity)]
    fn try_join_thread(
        &mut self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
    ) -> Result<ZombieThread, Result<Condvar, Error>> {
        match self.find_process_mut(pid).map_err(Err)? {
            ProcessRefMut::Running(process) => process.try_join_thread(tid),
            ProcessRefMut::Runnable(_) => {
                let reason: &str = "process is runnable";
                error!("join_thread(): {} (pid={:?}, tid={:?})", reason, pid, tid);
                Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)))
            },
            ProcessRefMut::Sleeping(_) => {
                let reason: &str = "process is sleeping";
                error!("join_thread(): {} (pid={:?}, tid={:?})", reason, pid, tid);
                Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)))
            },
            ProcessRefMut::Interrupted(_) => {
                let reason: &str = "process is interrupted";
                error!("join_thread(): {} (pid={:?}, tid={:?})", reason, pid, tid);
                Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)))
            },
            ProcessRefMut::Zombie(_) => {
                let reason: &str = "process is a zombie";
                error!("join_thread(): {} (pid={:?}, tid={:?})", reason, pid, tid);
                Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)))
            },
        }
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
    /// On success, the mutex that is associated with the given address is returned. If no mutex is
    /// associated with the given address, a new mutex is created and returned. On failure, an error
    /// is returned instead.
    ///
    fn get_mutex(&mut self, mutex_addr: MutexAddress) -> Result<Mutex, Error> {
        self.get_running_mut().state_mut().get_mutex(mutex_addr)
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
    fn get_cond(&mut self, cond_addr: ConditionAddress) -> Result<Condvar, Error> {
        self.get_running_mut().state_mut().get_cond(cond_addr)
    }

    ///
    /// # Description
    ///
    /// Releases a condition variable previously acquired by the calling thread.
    ///
    /// # Parameters
    ///
    /// - `cond_addr`: Address of the condition variable.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    fn put_cond(&mut self, cond_addr: ConditionAddress) -> Result<(), Error> {
        self.get_running_mut().state_mut().put_cond(cond_addr)
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
    fn put_mutex_guard(&mut self, mutex_addr: MutexAddress, guard: MutexGuard) {
        self.get_running_mut()
            .running_mut()
            .put_mutex_guard(mutex_addr, guard);
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
    fn take_mutex_guard(
        &mut self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        mutex_addr: MutexAddress,
    ) -> Result<MutexGuard, Error> {
        let mutex_guard: MutexGuard = match self
            .get_running_mut()
            .running_mut()
            .take_mutex_guard(mutex_addr)
        {
            Some(mutex_guard) => mutex_guard,
            None => {
                let reason: &str = "thread does not own mutex";
                error!("take_mutex_guard(): {} (pid={:?}, tid={:?})", reason, pid, tid);
                return Err(Error::new(ErrorCode::OperationNotPermitted, reason));
            },
        };

        self.get_running_mut().state_mut().put_mutex(mutex_addr)?;

        Ok(mutex_guard)
    }

    fn take_ready(&mut self) -> RunnableProcess {
        // NOTE: it is safe to call unwrap because there is always a process ready to run.
        self.ready
            .pop_front()
            .expect("the kernel should be ready to run")
    }

    fn take_running(&mut self) -> RunningProcess {
        // NOTE: it is safe to call unwrap because there is always a process running.
        self.running.take().expect("the kernel should be running")
    }

    fn get_running(&self) -> &RunningProcess {
        // NOTE: it is safe to call unwrap because there is always a process running.
        self.running.as_ref().expect("the kernel should be running")
    }

    fn get_running_mut(&mut self) -> &mut RunningProcess {
        // NOTE: it is safe to call unwrap because there is always a process running.
        self.running.as_mut().expect("the kernel should be running")
    }

    fn find_process(&self, pid: ProcessIdentifier) -> Result<ProcessRef, Error> {
        if self.get_running().state().pid() == pid {
            Ok(ProcessRef::Running(self.get_running()))
        } else if let Some(process) = self.ready.iter().find(|p| p.state().pid() == pid) {
            Ok(ProcessRef::Runnable(process))
        } else if let Some(process) = self.suspended.iter().find(|p| p.state().pid() == pid) {
            Ok(ProcessRef::Sleeping(process))
        } else if let Some(process) = self.interrupted.iter().find(|p| p.state().pid() == pid) {
            Ok(ProcessRef::Interrupted(process))
        } else if let Some(process) = self.zombies.iter().find(|p| p.state().pid() == pid) {
            Ok(ProcessRef::Zombie(process))
        } else {
            let reason: &str = "process not found";
            error!("find_process(): {} (pid={:?})", reason, pid);
            Err(Error::new(ErrorCode::NoSuchProcess, reason))
        }
    }

    fn find_process_mut(&mut self, pid: ProcessIdentifier) -> Result<ProcessRefMut, Error> {
        if self.get_running_mut().state().pid() == pid {
            Ok(ProcessRefMut::Running(self.get_running_mut()))
        } else if let Some(process) = self.ready.iter_mut().find(|p| p.state().pid() == pid) {
            Ok(ProcessRefMut::Runnable(process))
        } else if let Some(process) = self.suspended.iter_mut().find(|p| p.state().pid() == pid) {
            Ok(ProcessRefMut::Sleeping(process))
        } else if let Some(process) = self.interrupted.iter_mut().find(|p| p.state().pid() == pid) {
            Ok(ProcessRefMut::Interrupted(process))
        } else if let Some(process) = self.zombies.iter_mut().find(|p| p.state().pid() == pid) {
            Ok(ProcessRefMut::Zombie(process))
        } else {
            let reason: &str = "process not found";
            error!("find_process(): {} (pid={:?})", reason, pid);
            Err(Error::new(ErrorCode::NoSuchProcess, reason))
        }
    }
}

//==================================================================================================
// Process Manager
//==================================================================================================

pub struct ProcessManager(Rc<RefCell<ProcessManagerInner>>);

impl ProcessManager {
    ///
    /// # Description
    ///
    /// Returns the ID of the calling process.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the ID of the calling process is returned. Otherwise, an error
    /// code is returned instead.
    ///
    pub fn get_pid(&self) -> Result<ProcessIdentifier, Error> {
        // SAFETY: This is the only thread running, thus access to the process manager is synchronized.
        Ok(self.try_borrow()?.get_running().state().pid())
    }

    ///
    /// # Description
    ///
    /// Returns the ID of the calling thread.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the ID of the calling thread is returned. Otherwise, an error
    /// code is returned instead.
    ///
    pub fn get_tid(&self) -> Result<ThreadIdentifier, Error> {
        Ok(self.try_borrow()?.get_running().get_tid())
    }

    ///
    /// # Description
    ///
    /// Creates a new process.
    ///
    /// # Parameters
    ///
    /// - `mm`: Memory manager to use.
    /// - `elf`: ELF header of the executable to load.
    /// - `args`: Command line arguments.
    /// - `env`: Environment variables.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the process identifier of the new process is returned.
    /// Otherwise, an error is returned instead.
    ///
    pub fn create_process(
        &mut self,
        mm: &mut VirtMemoryManager,
        elf: &Elf32Fhdr,
        args: &str,
        env: &str,
    ) -> Result<ProcessIdentifier, Error> {
        self.try_borrow_mut()?.create_process(mm, elf, args, env)
    }

    /// Creates a new thread.
    pub fn create_thread(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        user_wrapper_fn: VirtualAddress,
        user_fn: VirtualAddress,
        user_fn_arg: usize,
    ) -> Result<ThreadIdentifier, Error> {
        self.try_borrow_mut()?
            .create_thread(mm, pid, user_wrapper_fn, user_fn, user_fn_arg)
    }

    pub fn has_capability(
        &self,
        pid: ProcessIdentifier,
        capability: Capability,
    ) -> Result<bool, Error> {
        Ok(self
            .try_borrow()?
            .find_process(pid)?
            .state()
            .has_capability(capability))
    }

    pub fn capctl(
        &mut self,
        pid: ProcessIdentifier,
        capability: Capability,
        value: bool,
    ) -> Result<(), Error> {
        self.try_borrow_mut()?.capctl(pid, capability, value)
    }

    pub fn terminate(&mut self, pid: ProcessIdentifier) -> Result<(), Error> {
        self.try_borrow_mut()?.terminate(pid)
    }

    pub fn vmcopy_from_user(
        &mut self,
        pid: ProcessIdentifier,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .copy_from_user_unaligned(dst, src, size)
    }

    pub fn vmcopy_to_user(
        &mut self,
        pid: ProcessIdentifier,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .copy_to_user_unaligned(dst, src, size)
    }

    pub fn harvest_zombies(
        &mut self,
        mm: &mut VirtMemoryManager,
    ) -> Result<Option<(ProcessIdentifier, ExitStatus)>, Error> {
        let (mut zombie_threads, mut state, status): (
            VecDeque<ZombieThread>,
            Box<ProcessState>,
            ExitStatus,
        ) = match self.try_borrow_mut()?.harvest_zombies() {
            Some((zombie_threads, state, status)) => (zombie_threads, state, status),
            None => return Ok(None),
        };

        // Traverse the list of zombie threads.
        while let Some(zombie_thread) = zombie_threads.pop_front() {
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
                    if let Err(error) = mm.unmap_upage(state.vmem_mut(), vaddr) {
                        // We failed, but this is not too bad, as we will free all pages
                        // when wiping out the address space anyways.
                        warn!(
                            "harvest_zombies(): failed to unmap page (vaddr={:?}, error={:?})",
                            vaddr, error
                        );
                    }
                }

                // Frames allocated to the user stack are freed when we exit this scope.
                // Frames allocated to the kernel stack are freed when we exit this scope.
            }
        }

        Ok(Some((state.pid(), status)))
    }

    pub fn mmap(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        let vmem: &mut Vmem = process.state_mut().vmem_mut();
        mm.alloc_upage(vmem, vaddr, access, true)
    }

    pub fn munmap(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        let vmem: &mut Vmem = process.state_mut().vmem_mut();
        mm.unmap_upage(vmem, vaddr)
    }

    pub fn mctrl(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        let vmem: &mut Vmem = process.state_mut().vmem_mut();
        mm.ctrl_upage(vmem, vaddr, access)
    }

    pub fn mmio_alloc(
        &mut self,
        pid: ProcessIdentifier,
        region: IoMemoryRegion,
    ) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        let state: &mut ProcessState = process.state_mut();

        // TODO: change page permissions.
        let vmem: &mut Vmem = state.vmem_mut();
        vmem.kctrl(region.base(), region.perm())?;

        state.add_mmio(region);

        Ok(())
    }

    pub fn mmio_free(
        &mut self,
        pid: ProcessIdentifier,
        addr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        let state: &mut ProcessState = process.state_mut();
        state.remove_mmio(addr);

        Ok(())
    }

    pub fn attach_pmio(&mut self, pid: ProcessIdentifier, port: AnyIoPort) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        process.state_mut().add_pmio(port);
        Ok(())
    }

    pub fn detach_pmio(
        &mut self,
        pid: ProcessIdentifier,
        port_number: u16,
    ) -> Result<AnyIoPort, Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        process.state_mut().remove_pmio(port_number)
    }

    pub fn read_pmio(
        &mut self,
        pid: ProcessIdentifier,
        port_number: u16,
        port_width: IoPortWidth,
    ) -> Result<u32, Error> {
        let pm: Ref<ProcessManagerInner> = self.try_borrow()?;
        let process: ProcessRef = pm.find_process(pid)?;
        process.state().read_pmio(port_number, port_width)
    }

    pub fn write_pmio(
        &mut self,
        pid: ProcessIdentifier,
        port_number: u16,
        port_width: IoPortWidth,
        value: u32,
    ) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        process
            .state_mut()
            .write_pmio(port_number, port_width, value)
    }

    ///
    /// # Description
    ///
    /// Sends a message to a process.
    ///
    /// # Parameters
    ///
    /// - `pid`: ID of the target process.
    /// - `message`: Message to send.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    pub fn post_message(&mut self, pid: ProcessIdentifier, message: Message) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        process.state_mut().post_message(message);
        pm.number_buffered_messages += 1;
        Ok(())
    }

    pub fn add_event(&mut self, ownership: EventOwnership) -> Result<(), Error> {
        self.try_borrow_mut()?
            .get_running_mut()
            .state_mut()
            .add_event(ownership);

        Ok(())
    }

    pub fn remove_event(&mut self, ev: &Event) -> Result<(), Error> {
        self.try_borrow_mut()?
            .get_running_mut()
            .state_mut()
            .remove_event(ev);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Returns the number of buffered messages.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of buffered messages is returned. Otherwise, an error
    /// code is returned instead.
    ///
    #[cfg(feature = "stdio")]
    pub fn number_buffered_messages(&self) -> Result<usize, Error> {
        Ok(self.try_borrow()?.number_buffered_messages)
    }

    fn try_borrow(&self) -> Result<Ref<ProcessManagerInner>, Error> {
        match self.0.try_borrow() {
            Ok(pm) => Ok(pm),
            Err(_) => {
                let reason: &str = "cannot borrow process manager";
                error!("try_borrow(): {}", reason);
                Err(Error::new(ErrorCode::ResourceBusy, reason))
            },
        }
    }

    fn try_borrow_mut(&mut self) -> Result<RefMut<ProcessManagerInner>, Error> {
        match self.0.try_borrow_mut() {
            Ok(pm) => Ok(pm),
            Err(_) => {
                let reason: &str = "cannot borrow process manager";
                error!("try_borrow_mut(): {}", reason);
                Err(Error::new(ErrorCode::ResourceBusy, reason))
            },
        }
    }
}
