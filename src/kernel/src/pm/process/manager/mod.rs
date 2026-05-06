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
            MmioTag,
        },
        mem::{
            AccessPermission,
            Address,
            PageAligned,
            VirtualAddress,
        },
        platform,
    },
    mm::{
        elf::Elf32Fhdr,
        kstack::KernelStack,
        ustack::UserStack,
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
            ThreadRef,
            ThreadRefMut,
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
    vec::Vec,
};
use ::arch::{
    cpu::excp,
    mem::PAGE_SIZE,
};
use ::config::memory_layout::{
    USER_STACK_MIN_SIZE,
    USER_STACK_SIZE,
    USER_STACK_TOP_RAW,
};
use ::no_fail::no_fail;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    event::Event,
    ipc::{
        Message,
        MessageReceiver,
    },
    pm::{
        Capability,
        ConditionAddress,
        MutexAddress,
        ProcessIdentifier,
        ThreadCreateArgs,
        ThreadIdentifier,
    },
    time::SystemTime,
    ExitStatus,
};
use ::type_safe::NonEmptyVecDeque;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::r#unsafe::ExceptionGuard;

//==================================================================================================
// Sleep Error
//==================================================================================================

#[derive(Debug)]
pub enum SleepError {
    Interrupted(InterruptReason),
    Generic(Error),
}

//==================================================================================================
// Process Manager
//==================================================================================================

///
/// # Description
///
/// A type that represents the process manager.
///
pub struct ProcessManager {
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

impl ProcessManager {
    /// Initializes the process manager.
    pub fn new(
        interrupt_capable: bool,
        kernel: ReadyThread,
        root: Vmem,
        tm: ThreadManager,
    ) -> Self {
        let kernel: RunnableProcess = RunnableProcess::new(ProcessIdentifier::KERNEL, kernel, root);

        let (kernel, reason, _, _user_tda): (
            RunningProcess,
            Option<InterruptReason>,
            *mut ContextInformation,
            Option<VirtualAddress>,
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

    ///
    /// # Description
    ///
    /// Creates a new thread in the calling process.
    ///
    /// # Parameters
    ///
    /// - `pm`: Handler to the process manager.
    /// - `mm`: Handler to the virtual memory manager.
    /// - `user_stack`: User stack to use for the new thread.
    /// - `user_fn`: User function to execute in the new thread.
    /// - `arg0`: First argument to pass to the user function.
    /// - `arg1`: Second argument to pass to the user function.
    /// - `enable_interrupts`: Whether to enable interrupts in the new thread.
    ///
    /// # Returns
    ///
    /// On successful completion, this function returns the thread identifier of the newly created
    /// thread.  On failure, it returns an error object that provides details about the failure.
    ///
    /// # Safety Notes
    ///
    /// - `user_stack_base` refers to a memory region that lies within the user address space, it is
    ///   writable.
    /// - `user_stack_size` is a multiple of `PAGE_SIZE`.
    /// - `user_fn` lies within the user address space and points to an executable memory region.
    ///
    fn forge_user_context(
        mm: &mut VirtMemoryManager,
        vmem: &mut Vmem,
        args: &ThreadCreateArgs,
        enable_interrupts: bool,
    ) -> Result<(KernelStack, ContextInformation), Error> {
        trace!("args={args:?}, enable_interrupts={enable_interrupts:?}",);

        unsafe extern "C" {
            pub fn __leave_kernel_to_user_mode();
        }

        // Assert pre-conditions (these should have been checked by the caller).
        debug_assert!(Vmem::is_user_region(args.user_stack_base, args.user_stack_size));
        debug_assert!(Vmem::is_user_addr(args.user_fn));

        let kernel_func: VirtualAddress =
            VirtualAddress::from_raw_value(__leave_kernel_to_user_mode as *const () as usize);

        // Alloc kernel pages for the kernel stack. If we fail beyond this point, `kernel_stack`
        // gets dropped as soon as we exit this scope and underlying pages are released.
        let kernel_stack: KernelStack = KernelStack::new(mm)?;

        let cr3: u32 = vmem.pgdir().physical_address()?.into_raw_value() as u32;
        let esp: u32 = unsafe {
            hal::arch::forge_user_stack(
                kernel_stack.top().into_raw_value() as *mut u8,
                args.user_stack_base.into_raw_value() + args.user_stack_size,
                args.user_fn.into_raw_value(),
                args.user_fn_arg0,
                args.user_fn_arg1,
                kernel_func.into_raw_value(),
                enable_interrupts,
            )
        } as u32;
        let esp0: u32 = kernel_stack.top().into_raw_value() as u32;

        trace!("cr3={:#x}, esp={:#x}, ebp={:#x}", cr3, esp, esp0);
        let context: ContextInformation = ContextInformation::new(cr3, esp, esp0);

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
    /// - `thread_create_args`: Arguments for the thread creation.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the thread identifier of the new thread is returned.
    /// Otherwise, an error is returned instead.
    ///
    /// # Safety Notes
    ///
    /// - `thread_create_args` must have valid fields, specifically:
    ///   - `user_wrapper_fn` must point to a user memory region that is executable.
    ///   - `user_fn` must point to a user memory region that is executable.
    ///   - `user_stack` must point to a user memory region that is writable.
    ///
    pub fn create_thread(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        thread_create_args: &ThreadCreateArgs,
    ) -> Result<ThreadIdentifier, Error> {
        trace!("pid={pid:?}, thread_create_args={thread_create_args:?}");

        // Assert pre-conditions (these should have been checked by the caller).
        debug_assert!(Vmem::is_user_addr(thread_create_args.user_fn));
        debug_assert!(
            self.get_running().state().pid() == pid,
            "create_thread: pid must match the running process"
        );

        // Reserve the next thread identifier early, before any resource allocation.
        let (tid, next_tid): (ThreadIdentifier, ThreadIdentifier) = self.tm.try_next_tid()?;

        let enable_interrupts: bool = self.interrupt_capable;

        // Create a kernel context.
        let (kernel_stack, context): (KernelStack, ContextInformation) = Self::forge_user_context(
            mm,
            self.get_running_mut().state_mut().vmem_mut(),
            thread_create_args,
            enable_interrupts,
        )?;

        //==============================================================
        // NOTE: if we fail beyond this point we need to page mappings.
        //==============================================================

        Ok(no_fail!(ThreadIdentifier, {
            // Create a new thread.
            let ready_thread: ReadyThread = self.tm.create_thread(
                tid,
                Some(kernel_stack),
                None,
                thread_create_args.user_tda,
                context,
            );

            // Commit the next thread identifier now that all fallible operations have succeeded.
            self.tm.commit_next_tid(next_tid);

            // Add the new thread to the running process.
            let tid: ThreadIdentifier = ready_thread.id();
            self.get_running_mut().add_thread(ready_thread);

            Ok(tid)
        }))
    }

    ///
    /// # Description
    ///
    /// Sets the base address for the user-space thread data area of a thread.
    ///
    /// # Parameters
    ///
    /// - `pid`: The identifier of the process containing the thread.
    /// - `tid`: The identifier of the thread whose thread data area pointer is to be set.
    /// - `user_tda`: Optional thread data area pointer to set.
    ///
    /// # Return Values
    ///
    /// Upon successful completion, this function return empty. Upon failure, this function returns
    /// an error.
    ///
    /// # Errors
    ///
    /// This function fails with the following error codes:
    ///
    /// - [`ErrorCode::NoSuchEntry`]: The specified process or thread does not exist.
    ///
    pub fn set_thread_data_area(
        &mut self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        user_tda: Option<VirtualAddress>,
    ) -> Result<(), Error> {
        // Search for the process across all states.
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;

        // Search for the thread and set its data area.
        match process.find_thread_mut(tid) {
            Some(ThreadRefMut::Sleeping(thread)) => {
                thread.set_thread_data_area(user_tda);
                Ok(())
            },
            Some(ThreadRefMut::Running(thread)) => {
                thread.set_thread_data_area(user_tda);
                Ok(())
            },
            _ => {
                let reason: &str = "thread not found";
                error!("{reason} (tid={tid:?}, pid={pid:?}, user_tda={user_tda:?})");
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Gets the based address for the user-space thread data area of a thread.
    ///
    /// # Parameters
    ///
    /// - `pid`: The identifier of the process containing the thread.
    /// - `tid`: The identifier of the thread whose thread data area pointer is to be retrieved.
    ///
    /// # Return Values
    ///
    /// Upon successful completion, this function returns the based-address for the user-space
    /// thread data area of the specified thread. Upon failure, this function returns an error.
    ///
    /// # Errors
    ///
    /// This function fails with the following error codes:
    ///
    /// - [`ErrorCode::NoSuchEntry`]: The specified process or thread does not exist.
    ///
    pub fn get_thread_data_area(
        &self,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
    ) -> Result<Option<VirtualAddress>, Error> {
        // Search for the process across all states.
        let process: ProcessRef = self.find_process(pid)?;

        // Search for the thread and get its data area.
        match process.find_thread(tid) {
            Some(ThreadRef::Sleeping(thread)) => Ok(thread.get_thread_data_area()),
            Some(ThreadRef::Running(thread)) => Ok(thread.get_thread_data_area()),
            _ => {
                let reason: &str = "thread not found";
                error!("{reason} (tid={tid:?}, pid={pid:?})");
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Writes a NUL-terminated string directly to user space without heap allocation.
    /// The string bytes are copied from the source `&str` followed by a single `\0` terminator.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory address space to write into.
    /// - `dest`: Destination virtual address in user space.
    /// - `s`: Source string to write (must not contain interior NUL bytes).
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    fn write_nul_terminated_to_user(
        vmem: &mut Vmem,
        dest: VirtualAddress,
        s: &str,
    ) -> Result<(), Error> {
        if !s.is_empty() {
            vmem.copy_to_user_unaligned(dest, VirtualAddress::new(s.as_ptr() as usize), s.len())?;
        }
        static NUL: u8 = 0;
        let nul_vaddr: VirtualAddress = VirtualAddress::new(dest.into_raw_value() + s.len());
        vmem.copy_to_user_unaligned(nul_vaddr, VirtualAddress::new(&NUL as *const u8 as usize), 1)?;
        Ok(())
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
    pub fn create_process(
        &mut self,
        mm: &mut VirtMemoryManager,
        elf: &Elf32Fhdr,
        args: &str,
        env: &str,
    ) -> Result<ProcessIdentifier, Error> {
        unsafe extern "C" {
            pub fn __leave_kernel_to_user_mode();
        }

        trace!("args={:?}, env={:?}", args, env);

        // Reserve the next process and thread identifiers early, before any resource allocation.
        let (pid, next_pid): (ProcessIdentifier, ProcessIdentifier) = self.try_next_pid()?;
        let (tid, next_tid): (ThreadIdentifier, ThreadIdentifier) = self.tm.try_next_tid()?;

        // Strip leading and trailing spaces from arguments.
        let args: &str = args.trim();

        // Validate that args does not contain interior null bytes (for C-string semantics).
        if args.as_bytes().contains(&0) {
            let reason: &str = "command line contains interior null byte";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        // args_len includes the null terminator that will be written to user space.
        let args_total_len: usize = args.len() + 1;

        // Strip leading and trailing spaces from environment variables.
        let env: &str = env.trim();

        // Validate that env does not contain interior null bytes (for C-string semantics).
        if env.as_bytes().contains(&0) {
            let reason: &str = "environment string contains interior null byte";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        // env_len includes the null terminator that will be written to user space.
        let env_total_len: usize = env.len() + 1;

        // Create a new memory address space for the process.
        let mut vmem: Vmem = mm.new_vmem(self.get_running().state().vmem())?;

        // Load the ELF file into the new address space.
        let (entry, args_vaddr): (VirtualAddress, PageAligned<VirtualAddress>) =
            mm.load_elf(&mut vmem, elf)?;

        // Allocate a user-space page, write command line arguments to it, and check for errors.
        // The total length includes the null terminator written after the args bytes, and must fit
        // entirely within a single page.
        if args_total_len > PAGE_SIZE {
            let reason: &str = "command line is too long";
            error!("{reason} (cmdline.len={:?})", args_total_len);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        mm.alloc_upages(
            &mut vmem,
            args_vaddr,
            AccessPermission::RDWR,
            true,
            1,
            &mut Vec::with_capacity(1),
        )?;
        // Write args as a NUL-terminated string directly to user space.
        Self::write_nul_terminated_to_user(&mut vmem, args_vaddr.into_inner(), args)?;
        debug!(
            "arguments written to user space (args_vaddr={:?}, args={:?})",
            args_vaddr,
            args.as_bytes()
        );

        // Allocate another page for the environment variables and check for errors.
        // The total length includes the null terminator and must fit within a single page.
        if env_total_len > PAGE_SIZE {
            let reason: &str = "environment variables are too long";
            error!("{reason} (env.len={:?})", env_total_len);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let envp_vaddr: PageAligned<VirtualAddress> = PageAligned::<VirtualAddress>::from_address(
            VirtualAddress::new(args_vaddr.into_raw_value() + PAGE_SIZE),
        )?;
        mm.alloc_upages(
            &mut vmem,
            envp_vaddr,
            AccessPermission::RDWR,
            true,
            1,
            &mut Vec::with_capacity(1),
        )?;

        // Write env as a NUL-terminated string directly to user space.
        Self::write_nul_terminated_to_user(&mut vmem, envp_vaddr.into_inner(), env)?;
        debug!(
            "environment variables written to user space (envp_vaddr={:?}, env={:?})",
            envp_vaddr,
            env.as_bytes()
        );

        // Create a kernel context.
        let user_stack: UserStack =
            UserStack::new(PageAligned::from_raw_value(USER_STACK_TOP_RAW)?);
        let user_fn: VirtualAddress = entry;
        let argp: usize = args_vaddr.into_raw_value();
        let envp: usize = envp_vaddr.into_raw_value();

        let args: ThreadCreateArgs = ThreadCreateArgs {
            user_fn,
            user_fn_arg0: argp,
            user_fn_arg1: envp,
            user_stack_base: user_stack.base().into_inner(),
            user_stack_size: user_stack.size(),
            user_tda: None, // The base address for the user-space thread data area the main thread is set by the user-space runtime.
        };

        let (kernel_stack, context): (KernelStack, ContextInformation) =
            Self::forge_user_context(mm, &mut vmem, &args, self.interrupt_capable)?;

        // Map only the minimum number of stack pages near the stack top (where ESP starts).
        // Additional pages up to USER_STACK_SIZE are demand-paged on stack growth faults.
        // NOTE: if we fail beyond this point we must unmap kernel pages from `vmem`, otherwise we
        // will leak underlying pages.
        let initial_stack_base: PageAligned<VirtualAddress> =
            PageAligned::from_raw_value(user_stack.top().into_raw_value() - USER_STACK_MIN_SIZE)?;
        let count = USER_STACK_MIN_SIZE / PAGE_SIZE;
        mm.alloc_upages(
            &mut vmem,
            initial_stack_base,
            AccessPermission::RDWR,
            true,
            count,
            &mut Vec::with_capacity(count),
        )?;

        //==============================================================
        // NOTE: if we fail beyond this point we need to page mappings.
        //==============================================================

        Ok(no_fail!(ProcessIdentifier, {
            let thread: ReadyThread = self.tm.create_thread(
                tid,
                Some(kernel_stack),
                Some(user_stack),
                args.user_tda,
                context,
            );

            // Commit the next process and thread identifiers now that all fallible operations have succeeded.
            self.next_pid = next_pid;
            self.tm.commit_next_tid(next_tid);
            let process: RunnableProcess = RunnableProcess::new(pid, thread, vmem);

            // Add process to the queue of ready processes.
            self.ready.push_back(process);

            Ok(pid)
        }))
    }

    ///
    /// # Description
    ///
    /// Schedule a thread to run.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing:
    /// - The process identifier of the next thread to run.
    /// - The thread identifier of the next thread to run.
    /// - A pointer to the context information of the previous thread.
    /// - A pointer to the context information of the next thread.
    /// - An optional base address for the user-space thread data area of the next thread to run.
    ///
    fn schedule(
        &mut self,
    ) -> (
        ProcessIdentifier,
        ThreadIdentifier,
        *mut ContextInformation,
        *mut ContextInformation,
        Option<VirtualAddress>,
    ) {
        // Check the running thread's kernel stack guard watermark before switching away.
        self.check_running_stack_guard();

        // Reschedule running process.
        let previous_process: RunningProcess = self.take_running();

        let (previous_process, previous_context) = previous_process.schedule();
        self.ready.push_back(previous_process);

        self.check_alarm();

        // Process all interrupted processes.
        while let Some(interrupted_process) = self.interrupted.pop_front() {
            let ready_process: RunnableProcess = interrupted_process.resume();
            self.ready.push_back(ready_process);
        }

        // Select next process to run.
        let next_process: RunnableProcess = self.take_earliest_ready();

        let (next_process, reason, next_context, user_tda): (
            RunningProcess,
            Option<InterruptReason>,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = next_process.run();

        let next_pid: ProcessIdentifier = next_process.state().pid();
        let next_tid: ThreadIdentifier = next_process.get_tid();
        self.interrupt_reason = reason;
        self.running = Some(next_process);
        self.update_active_stack_guard();
        (next_pid, next_tid, previous_context, next_context, user_tda)
    }

    // Traverses the list of sleeping processes, checking for expired alarms and moving processes
    // whose alarms have expired from the `suspended` to `interrupted` list.
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
                        "process {:?} interrupted at {now:?}",
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
    /// Suspends the execution of the calling thread and schedules another thread to run.
    ///
    /// # Parameters
    ///
    /// - `alarm`: Optional alarm time.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing:
    /// - The process identifier of the next thread to run.
    /// - The thread identifier of the next thread to run.
    /// - A pointer to the context information of the previous thread.
    /// - A pointer to the context information of the next thread.
    /// - An optional base address for the user-space thread data area of the next thread to run.
    ///
    fn do_sleep(
        &mut self,
        alarm: Option<SystemTime>,
    ) -> (
        ProcessIdentifier,
        ThreadIdentifier,
        *mut ContextInformation,
        *mut ContextInformation,
        Option<VirtualAddress>,
    ) {
        // Check the running thread's kernel stack guard watermark before switching away.
        self.check_running_stack_guard();

        let running_process: RunningProcess = self.take_running();

        // Check if kernel is trying to sleep.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot sleep");
        }

        // Suspend the execution of the calling thread.
        let previous_context: *mut ContextInformation = match running_process.sleep(alarm) {
            // The calling process still has runnable threads, put it in the list of ready processes.
            Ok((runnable_process, previous_context)) => {
                self.ready.push_back(runnable_process);
                previous_context
            },
            // The calling process has only sleeping threads left, put it in the list of suspended processes.
            Err((suspended_process, previous_context)) => {
                self.suspended.push_back(suspended_process);
                previous_context
            },
        };

        // Schedule another thread to run.
        let next_process: RunnableProcess = self.take_earliest_ready();

        let (next_process, reason, next_context, user_tda): (
            RunningProcess,
            Option<InterruptReason>,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = next_process.run();

        let next_pid: ProcessIdentifier = next_process.state().pid();
        let next_tid: ThreadIdentifier = next_process.get_tid();
        self.interrupt_reason = reason;
        self.running = Some(next_process);
        self.update_active_stack_guard();
        (next_pid, next_tid, previous_context, next_context, user_tda)
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
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    pub fn do_wakeup(&mut self, tid: ThreadIdentifier) -> Result<(), Error> {
        // Check if thread belongs to the running process.
        if self.get_running().find_thread(tid).is_some() {
            let running_process: RunningProcess = self.take_running();
            match running_process.wakeup(tid) {
                Ok(running_process) => {
                    self.running = Some(running_process);
                    return Ok(());
                },
                Err(running_process) => {
                    self.running = Some(running_process);
                    let reason: &str = "thread not found";
                    error!("{reason} (tid={tid:?})");
                    return Err(Error::new(ErrorCode::NoSuchEntry, reason));
                },
            }
        }

        // Check if thread belongs to a suspended process.
        let runnable_process: RunnableProcess = match self.try_wakeup(tid) {
            Some(runnable_process) => runnable_process,
            None => {
                let reason: &str = "thread not found";
                error!("{reason} (tid={tid:?})");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            },
        };

        self.ready.push_back(runnable_process);

        Ok(())
    }

    fn try_wakeup(&mut self, tid: ThreadIdentifier) -> Option<RunnableProcess> {
        // Search for the process in the list of sleeping processes.
        let mut suspended: LinkedList<SleepingProcess> = LinkedList::new();
        while let Some(process) = self.suspended.pop_front() {
            // Found.
            if process.find_thread(tid).is_some() {
                match process.wakeup(tid) {
                    Ok(runnable_process) => {
                        while let Some(process) = suspended.pop_back() {
                            self.suspended.push_front(process);
                        }
                        return Some(runnable_process);
                    },
                    Err(suspended_process) => {
                        self.suspended.push_front(suspended_process);
                        while let Some(process) = suspended.pop_back() {
                            self.suspended.push_front(process);
                        }
                        return None;
                    },
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
            if process.find_thread(tid).is_some() {
                match process.wakeup(tid) {
                    Ok(runnable_process) => {
                        while let Some(process) = ready.pop_back() {
                            self.ready.push_front(process);
                        }
                        return Some(runnable_process);
                    },
                    Err(ready_process) => {
                        self.ready.push_front(ready_process);
                        while let Some(process) = ready.pop_back() {
                            self.ready.push_front(process);
                        }
                        return None;
                    },
                }
            } else {
                ready.push_back(process)
            }
        }
        // Process is not in the list of ready processes, rollback list to its original state.
        self.ready = ready;

        None
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
    /// Returns a tuple containing:
    /// - The process identifier of the next thread to run.
    /// - The thread identifier of the next thread to run.
    /// - A pointer to the context information of the previous thread.
    /// - A pointer to the context information of the next thread.
    /// - An optional base address for the user-space thread data area of the next thread to run.
    ///
    fn do_exit(
        &mut self,
        status: ExitStatus,
    ) -> (
        ProcessIdentifier,
        ThreadIdentifier,
        *mut ContextInformation,
        *mut ContextInformation,
        Option<VirtualAddress>,
    ) {
        // Check the running thread's kernel stack guard watermark before switching away.
        self.check_running_stack_guard();

        let running_process: RunningProcess = self.take_running();
        trace!(
            "pid={:?}, tid={:?}, status={status:?}",
            running_process.state().pid(),
            running_process.get_tid(),
        );

        // Check if kernel is trying to exit.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot exit");
        }

        // Clean up any pending rendezvous entries for this process and wake up counterpart
        // threads that would otherwise block forever.
        self.cleanup_rendezvous(running_process.state().pid(), "do_exit");

        // Terminate the calling thread.
        let previous_context: *mut ContextInformation = match running_process.exit(status) {
            // The calling process still has runnable threads, put it in the list of ready processes.
            Ok((runnable_process, previous_context)) => {
                self.ready.push_back(runnable_process);
                previous_context
            },
            // The calling process has only sleeping threads left, put it in the list of zombies processes.
            Err((zombie_process, previous_context)) => {
                self.zombies.push_back(zombie_process);
                previous_context
            },
        };

        // Schedule another thread to run.
        let next_process: RunnableProcess = self.take_earliest_ready();

        let (next_process, reason, next_context, user_tda): (
            RunningProcess,
            Option<InterruptReason>,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = next_process.run();

        let next_pid: ProcessIdentifier = next_process.state().pid();
        let next_tid: ThreadIdentifier = next_process.get_tid();
        self.interrupt_reason = reason;
        self.running = Some(next_process);
        self.update_active_stack_guard();
        (next_pid, next_tid, previous_context, next_context, user_tda)
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
    /// Returns a tuple containing:
    /// - The process identifier of the next thread to run.
    /// - The thread identifier of the next thread to run.
    /// - A pointer to the context information of the previous thread.
    /// - A pointer to the context information of the next thread.
    /// - An optional base address for the user-space thread data area of the next thread to run.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it may panic.
    ///
    /// This function is safe to use if and only if the following conditions are met:
    ///
    /// - The calling process is not the kernel process.
    ///
    fn do_exit_thread(
        &mut self,
        status: ExitStatus,
    ) -> (
        ProcessIdentifier,
        ThreadIdentifier,
        Condvar,
        *mut ContextInformation,
        *mut ContextInformation,
        Option<VirtualAddress>,
    ) {
        // Check the running thread's kernel stack guard watermark before switching away.
        self.check_running_stack_guard();

        let running_process: RunningProcess = self.take_running();

        trace!(
            "pid={:?}, tid={:?}, status={:?}",
            running_process.state().pid(),
            running_process.get_tid(),
            status
        );

        // Check if kernel is trying to exit.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot exit (status={status:?})");
        }

        // Terminate the calling thread and schedule another thread to run.
        let (join_cond, previous_context): (Condvar, *mut ContextInformation) =
            match running_process.exit_thread(status) {
                // The calling process still has runnable threads, put it in the list of ready processes.
                Ok((join_cond, runnable_process, previous_context)) => {
                    self.ready.push_back(runnable_process);
                    (join_cond, previous_context)
                },
                // The calling process has only sleeping threads left, put it in the list of suspended processes.
                Err(Ok((join_cond, sleeping_process, previous_context))) => {
                    self.suspended.push_back(sleeping_process);
                    (join_cond, previous_context)
                },
                // The calling process has only zombie threads left, put it in the list of zombies processes.
                Err(Err((join_cond, zombie_process, previous_context))) => {
                    self.zombies.push_back(zombie_process);
                    (join_cond, previous_context)
                },
            };

        // Schedule another thread to run.
        let next_process: RunnableProcess = self.take_earliest_ready();

        let (next_process, reason, next_context, user_tda): (
            RunningProcess,
            Option<InterruptReason>,
            *mut ContextInformation,
            Option<VirtualAddress>,
        ) = next_process.run();

        let next_pid: ProcessIdentifier = next_process.state().pid();
        let next_tid: ThreadIdentifier = next_process.get_tid();
        self.interrupt_reason = reason;
        self.running = Some(next_process);
        self.update_active_stack_guard();
        (next_pid, next_tid, join_cond, previous_context, next_context, user_tda)
    }

    pub fn terminate(&mut self, pid: ProcessIdentifier) -> Result<(), Error> {
        // Check if terminating kernel process.
        if pid == ProcessIdentifier::KERNEL {
            let reason: &str = "cannot terminate kernel process";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Check if target process is running.
        if self.running.is_some() && self.get_running().state().pid() == pid {
            let reason: &str = "cannot terminate running process";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // Clean up any pending rendezvous entries for the terminated process and wake up
        // counterpart threads that would otherwise block forever.
        self.cleanup_rendezvous(pid, "terminate");

        // Check if target process is ready.
        if let Some(process) = self.ready.iter().position(|p| p.state().pid() == pid) {
            let process: RunnableProcess = self.ready.remove(process);
            match process.terminate() {
                Ok(interrupted_process) => {
                    let runnable_process: RunnableProcess = interrupted_process.resume();
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
        error!("{reason}");
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

        // Check whether the capability should be set or cleared.
        if set {
            // Check if capability is already set.
            if process.state_mut().has_capability(capability) {
                let reason: &str = "capability already set";
                error!("{reason}");
                return Err(Error::new(ErrorCode::ResourceBusy, reason));
            }
            process.state_mut().set_capability(capability);
        } else {
            // Check if capability is not set.
            if !process.state_mut().has_capability(capability) {
                let reason: &str = "capability not set";
                error!("{reason}");
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            }
            process.state_mut().clear_capability(capability);
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Handles an FPU exception by saving and restoring the FPU state of the current and previous
    /// FPU owner threads.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    pub fn handle_fpu_exception(&mut self) -> Result<(), Error> {
        use crate::{
            hal::arch::x86::cpu::FpuState,
            pm::{
                process::manager::r#unsafe::{
                    CURRENT_TID,
                    FPU_OWNER_TID,
                },
                ORDER,
            },
        };

        // Clear CR0.TS to re-enable FPU/SSE instructions.
        unsafe { hal::arch::clear_task_switched() };

        let current_tid: ThreadIdentifier = CURRENT_TID.load(ORDER).into();

        let current_fpu_state: *mut FpuState =
            match self.get_running_mut().find_thread_mut(current_tid) {
                Some(mut running) => running.thread_state_mut().fpu_state_mut(),
                None => {
                    let reason: &str = "no running process";
                    error!("{reason} (tid={current_tid:?})");
                    return Err(Error::new(ErrorCode::NoSuchEntry, reason));
                },
            };

        let previous_fpu_owner: ThreadIdentifier = FPU_OWNER_TID.load(ORDER).into();

        let previous_fpu_state: Option<*mut FpuState> = if previous_fpu_owner == current_tid {
            // Current thread is already the FPU owner, nothing to do.
            return Ok(());
        } else {
            match self.find_thread_mut(previous_fpu_owner) {
                Ok(mut thread) => Some(thread.thread_state_mut().fpu_state_mut()),
                _ => None,
            }
        };

        // Save the previous thread's FPU state if there was a previous owner that used FPU.
        if let Some(prev_state) = previous_fpu_state {
            unsafe { FpuState::save(prev_state) };
        }

        // Current thread has used FPU before, restore its state.
        unsafe { FpuState::restore(current_fpu_state) };

        FPU_OWNER_TID.store(current_tid.into(), ORDER);

        Ok(())
    }

    fn interrupt_reason(&mut self) -> Option<InterruptReason> {
        self.interrupt_reason.take()
    }

    fn pop_zombie_process(
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
                error!("{reason} (pid={pid:?}, tid={tid:?})");
                Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)))
            },
            ProcessRefMut::Sleeping(_) => {
                let reason: &str = "process is sleeping";
                error!("{reason} (pid={pid:?}, tid={tid:?})");
                Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)))
            },
            ProcessRefMut::Interrupted(_) => {
                let reason: &str = "process is interrupted";
                error!("{reason} (pid={pid:?}, tid={tid:?})");
                Err(Err(Error::new(ErrorCode::OperationNotPermitted, reason)))
            },
            ProcessRefMut::Zombie(_) => {
                let reason: &str = "process is a zombie";
                error!("{reason} (pid={pid:?}, tid={tid:?})");
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
    fn lookup_mutex(&mut self, mutex_addr: MutexAddress) -> Result<Mutex, Error> {
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
    fn lookup_cond(&mut self, cond_addr: ConditionAddress) -> Result<Condvar, Error> {
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
    fn release_cond(&mut self, cond_addr: ConditionAddress) -> Result<(), Error> {
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
    fn store_mutex_guard(&mut self, mutex_addr: MutexAddress, guard: MutexGuard) {
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
    fn remove_mutex_guard(
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
                error!("{reason} (pid={pid:?}, tid={tid:?})");
                return Err(Error::new(ErrorCode::OperationNotPermitted, reason));
            },
        };

        self.get_running_mut().state_mut().put_mutex(mutex_addr)?;

        Ok(mutex_guard)
    }

    ///
    /// # Description
    ///
    /// Reserves the next process identifier, performing a checked increment.
    ///
    /// # Returns
    ///
    /// Upon success, this function returns a tuple containing the reserved [`ProcessIdentifier`]
    /// and the next [`ProcessIdentifier`] value. The caller must commit the next identifier by
    /// updating `self.next_pid` after all fallible operations have succeeded.
    ///
    /// # Errors
    ///
    /// This function returns an error if the process identifier would overflow.
    ///
    /// # Known Bugs
    ///
    /// - FIXME (#1440): process identifiers are never recycled, so a fork bomb or repeated
    ///   `create_process` calls can exhaust the identifier space.
    ///
    fn try_next_pid(&self) -> Result<(ProcessIdentifier, ProcessIdentifier), Error> {
        let pid: ProcessIdentifier = self.next_pid;
        let raw_pid: i32 = i32::from(pid);
        let next_raw_pid: i32 = match raw_pid.checked_add(1) {
            Some(val) => val,
            None => {
                let reason: &str = "process identifier overflow";
                error!("{reason} (next_pid={raw_pid:?})");
                return Err(Error::new(ErrorCode::ValueOverflow, reason));
            },
        };
        Ok((pid, ProcessIdentifier::from(next_raw_pid)))
    }

    fn take_earliest_ready(&mut self) -> RunnableProcess {
        // SAFETY: As the kernel process is always runnable, the following statement will never panic.
        let mut selected: (usize, SystemTime) = (
            0,
            self.ready
                .front()
                .expect("there should always be a process ready to run")
                .earliest_admission_time(),
        );

        // Select process with the earliest admission time.
        for (i, process) in self.ready.iter().enumerate() {
            let process_admission_time: SystemTime = process.earliest_admission_time();
            if process_admission_time < selected.1 {
                selected = (i, process_admission_time);
            }
        }

        // Remove the selected process from the list of ready processes.
        self.ready.remove(selected.0)
    }

    ///
    /// # Description
    ///
    /// Checks the running thread's kernel stack guard watermark for corruption.
    /// If corrupted, logs an error and halts the VM immediately.
    ///
    fn check_running_stack_guard(&self) {
        // Skip when serving an exception: the stack may already be overflowed and
        // the halt path would aggravate the situation by consuming more stack.
        if Self::is_serving_exception() {
            return;
        }

        if let Some(ref running) = self.running {
            if let Err(_e) = running.check_guard_watermark() {
                // Do NOT panic here. A panic formats debug information via core::fmt, which
                // allocates a large stack frame. When this function is called from do_schedule
                // (invoked by the timer interrupt handler), the kernel stack is already near its
                // limit. The additional stack consumed by panic formatting can corrupt page
                // directory entries, triggering a recursive exception cascade (triple fault).
                //
                // Instead, log a fixed-size message and halt immediately. The error! macro and
                // platform::shutdown use minimal stack compared to panic! formatting.
                error!(
                    "stack overflow detected: tid={:?}, pid={:?}",
                    running.get_tid(),
                    running.state().pid(),
                );
                platform::shutdown(ExitStatus::STACK_OVERFLOW_WATERMARK.into());
            }
        }
    }

    ///
    /// # Description
    ///
    /// Updates the assembly-level stack overflow guard to match the currently-active kernel stack.
    /// Called after setting `self.running` in every scheduling path.
    ///
    fn update_active_stack_guard(&self) {
        #[cfg(feature = "exception-stack-guard")]
        {
            if let Some(ref running) = self.running {
                if let Some(threshold) = running.guard_threshold() {
                    crate::mm::kstack::set_active_guard(threshold);
                    return;
                }
            }

            // When there is no running process or no guard threshold, clear the guard so that
            // a stale threshold from a previous stack does not trigger a false overflow.
            crate::mm::kstack::set_active_guard(0);
        }
    }

    ///
    /// # Description
    ///
    /// Cleans up pending rendezvous push/pull entries for a process and wakes up counterpart
    /// threads that would otherwise block forever. This is called during both voluntary exit
    /// (`do_exit`) and forced termination (`terminate`).
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier of the exiting/terminated process.
    /// - `caller`: Label used in log messages to identify the call site.
    ///
    fn cleanup_rendezvous(&mut self, pid: ProcessIdentifier, caller: &str) {
        // SAFETY: single-core system with interrupts disabled.
        let orphaned_tids: ::alloc::vec::Vec<ThreadIdentifier> =
            unsafe { crate::ipc::rendezvous::cleanup_process(pid) };
        for tid in orphaned_tids {
            if let Err(e) = self.do_wakeup(tid) {
                warn!(
                    "{caller}(): failed to wake orphaned rendezvous thread (tid={tid:?}, \
                     error={e:?})"
                );
            }
        }
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

    fn find_process(&self, pid: ProcessIdentifier) -> Result<ProcessRef<'_>, Error> {
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
            error!("{reason} (pid={pid:?})");
            Err(Error::new(ErrorCode::NoSuchProcess, reason))
        }
    }

    fn find_process_mut(&mut self, pid: ProcessIdentifier) -> Result<ProcessRefMut<'_>, Error> {
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
            error!("{reason} (pid={pid:?})");
            Err(Error::new(ErrorCode::NoSuchProcess, reason))
        }
    }

    ///
    /// # Description
    ///
    /// Finds a process by looking for a thread identifier.
    ///
    /// # Parameters
    ///
    /// - `tid`: Thread identifier to search for.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a mutable reference to the process containing the thread is
    /// returned.  Otherwise, an error code is returned instead.
    ///
    fn find_process_by_tid(&mut self, tid: ThreadIdentifier) -> Result<ProcessRefMut<'_>, Error> {
        if self.get_running_mut().find_thread(tid).is_some() {
            Ok(ProcessRefMut::Running(self.get_running_mut()))
        } else if let Some(process) = self.ready.iter_mut().find(|p| p.find_thread(tid).is_some()) {
            Ok(ProcessRefMut::Runnable(process))
        } else if let Some(process) = self
            .suspended
            .iter_mut()
            .find(|p| p.find_thread(tid).is_some())
        {
            Ok(ProcessRefMut::Sleeping(process))
        } else if let Some(process) = self
            .interrupted
            .iter_mut()
            .find(|p| p.find_thread(tid).is_some())
        {
            Ok(ProcessRefMut::Interrupted(process))
        } else if let Some(process) = self
            .zombies
            .iter_mut()
            .find(|p| p.find_thread(tid).is_some())
        {
            Ok(ProcessRefMut::Zombie(process))
        } else {
            let reason: &str = "thread not found";
            error!("{reason} (tid={tid:?})");
            Err(Error::new(ErrorCode::NoSuchEntry, reason))
        }
    }

    ///
    /// # Description
    ///
    /// Finds a thread.
    ///
    /// # Arguments
    ///
    /// - `tid`: Identifier of the thread to find.
    ///
    /// # Returns
    ///
    /// If a thread that matches the specified thread identifier is found, then a mutable reference
    /// to it is returned. Otherwise, empty is returned instead.
    ///
    fn find_thread_mut(&mut self, tid: ThreadIdentifier) -> Result<ThreadRefMut<'_>, Error> {
        // Search thread in the running process.
        if let Some(thread) = self.running.as_mut() {
            if let Some(thread) = thread.find_thread_mut(tid) {
                return Ok(thread);
            }
        }

        // Search thread in the list of ready processes.
        for process in self.ready.iter_mut() {
            if let Some(thread) = process.find_thread_mut(tid) {
                return Ok(thread);
            }
        }

        // Search thread in the list of sleeping processes.
        for process in self.suspended.iter_mut() {
            if let Some(thread) = process.find_thread_mut(tid) {
                return Ok(thread);
            }
        }

        // Search thread in the list of interrupted processes.
        for process in self.interrupted.iter_mut() {
            if let Some(thread) = process.find_thread_mut(tid) {
                return Ok(thread);
            }
        }

        // Search thread in the list of zombie processes.
        for process in self.zombies.iter_mut() {
            if let Some(thread) = process.find_thread_mut(tid) {
                return Ok(thread);
            }
        }

        let reason: &str = "thread not found";
        error!("{reason} (tid={tid:?})");
        Err(Error::new(ErrorCode::NoSuchEntry, reason))
    }

    ///
    /// # Description
    ///
    /// Notes that a message was posted.
    ///
    /// # Parameters
    ///
    /// None.
    ///
    /// # Returns
    ///
    /// An error if and only if incrementing by one would overflow the number of buffered messages.
    ///
    pub fn note_message_posted(&mut self) -> Result<(), Error> {
        match self.number_buffered_messages.checked_add(1) {
            Some(n) => {
                self.number_buffered_messages = n;
                Ok(())
            },
            None => {
                error!("number of buffered messages overflowed");
                Err(Error::new(ErrorCode::ValueOverflow, "number of buffered messages overflowed"))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Notes that a message was received.
    ///
    /// # Parameters
    ///
    /// None.
    ///
    /// # Returns
    ///
    /// An error if and only if decrementing by one would underflow the number of buffered messages.
    ///
    pub fn note_message_received(&mut self) -> Result<(), Error> {
        match self.number_buffered_messages.checked_sub(1) {
            Some(n) => {
                self.number_buffered_messages = n;
                Ok(())
            },
            None => {
                error!("number of buffered messages underflowed");
                Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "number of buffered messages underflowed",
                ))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Returns the number of messages that have been posted but not yet received.
    ///
    /// # Parameters
    ///
    /// None.
    ///
    /// # Returns
    ///
    /// The count of buffered messages.
    ///
    #[cfg(feature = "stdio")]
    pub fn number_buffered_messages(&self) -> usize {
        self.number_buffered_messages
    }

    ///
    /// # Description
    ///
    /// Returns the ID of the calling process.
    ///
    /// # Returns
    ///
    /// The ID of the calling process.
    ///
    pub fn get_pid(&self) -> ProcessIdentifier {
        self.get_running().state().pid()
    }

    ///
    /// # Description
    ///
    /// Returns the ID of the calling thread.
    ///
    /// # Returns
    ///
    /// The ID of the calling thread.
    ///
    pub fn get_tid(&self) -> ThreadIdentifier {
        self.get_running().get_tid()
    }

    pub fn has_capability(
        &self,
        pid: ProcessIdentifier,
        capability: Capability,
    ) -> Result<bool, Error> {
        Ok(self.find_process(pid)?.state().has_capability(capability))
    }

    pub fn vmcopy_from_user(
        &mut self,
        pid: ProcessIdentifier,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.find_process_mut(pid)?
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
        self.find_process_mut(pid)?
            .state_mut()
            .copy_to_user_unaligned(dst, src, size)
    }

    ///
    /// # Description
    ///
    /// Copies data directly between the user spaces of two processes.
    ///
    /// # Parameters
    ///
    /// - `src_pid`: Source process identifier.
    /// - `src`: Source address in `src_pid`'s user space.
    /// - `dst_pid`: Destination process identifier.
    /// - `dst`: Destination address in `dst_pid`'s user space.
    /// - `size`: Number of bytes to copy.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. On failure, an error is returned instead.
    ///
    pub fn vmcopy_user_to_user(
        &self,
        src_pid: ProcessIdentifier,
        src: VirtualAddress,
        dst_pid: ProcessIdentifier,
        dst: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        let src_proc: ProcessRef<'_> = self.find_process(src_pid)?;
        let dst_proc: ProcessRef<'_> = self.find_process(dst_pid)?;
        let src_vmem: &Vmem = src_proc.state().vmem();
        let dst_vmem: &Vmem = dst_proc.state().vmem();
        Vmem::copy_user_to_user(src_vmem, src, dst_vmem, dst, size)
    }

    ///
    /// # Description
    ///
    /// Translates a user-space virtual address to a guest physical address for a given process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process whose page tables should be walked.
    /// - `vaddr`: User-space virtual address to translate.
    ///
    /// # Returns
    ///
    /// Upon success, the guest physical address is returned. Upon failure, an error is returned.
    ///
    #[cfg(feature = "stdio")]
    pub fn user_vaddr_to_paddr(
        &self,
        pid: ProcessIdentifier,
        vaddr: VirtualAddress,
    ) -> Result<usize, Error> {
        let proc_ref: ProcessRef<'_> = self.find_process(pid)?;
        proc_ref.state().vmem().user_vaddr_to_paddr(vaddr)
    }

    pub fn harvest_zombies(
        &mut self,
        mm: &mut VirtMemoryManager,
    ) -> Result<Option<(ProcessIdentifier, ExitStatus)>, Error> {
        let (mut zombie_threads, mut state, status): (
            VecDeque<ZombieThread>,
            Box<ProcessState>,
            ExitStatus,
        ) = match self.pop_zombie_process() {
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
                    // Attempt to unmap page.
                    match mm.try_unmap_upage(state.vmem_mut(), vaddr) {
                        Ok(true) => {
                            // Page was present and has been successfully unmapped.
                        },
                        Ok(false) => {
                            // Page was never mapped (not demand-paged). Skip silently.
                        },
                        Err(error) => {
                            // Unexpected failure — log but continue since the
                            // address space will be reclaimed when it is destroyed.
                            warn!("failed to unmap page (vaddr={:?}, error={:?})", vaddr, error);
                        },
                    }
                }

                // Frames allocated to the user stack are freed when we exit this scope.
                // Frames allocated to the kernel stack are freed when we exit this scope.
            }
        }

        Ok(Some((state.pid(), status)))
    }

    ///
    /// # Description
    ///
    /// Maps one or more pages into the address space of a process.
    ///
    /// # Parameters
    ///
    /// - `mm`: Virtual memory manager.
    /// - `pid`: Process identifier.
    /// - `vaddr`: Page-aligned base virtual address to map.
    /// - `npages`: Number of pages to map.
    /// - `access`: Access permissions for the mapped pages.
    ///
    /// # Returns
    ///
    /// Upon successful completion, `Ok(())` is returned. Otherwise, an error is returned instead.
    ///
    pub fn mmap(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        vaddr: PageAligned<VirtualAddress>,
        npages: usize,
        access: AccessPermission,
    ) -> Result<(), Error> {
        /// Maximum number of pages to allocate per batch. Caps the Vec allocation on the kheap.
        const MMAP_BATCH_SIZE: usize = 16;

        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        let vmem: &mut Vmem = process.state_mut().vmem_mut();
        let mut current_vaddr: PageAligned<VirtualAddress> = vaddr;
        let mut remaining: usize = npages;

        while remaining > 0 {
            let count: usize = remaining.min(MMAP_BATCH_SIZE);
            let mut uframes = Vec::new();
            let batch: usize = if uframes.try_reserve(count).is_ok() {
                count
            } else if uframes.try_reserve(1).is_ok() {
                // Batch allocation failed; fall back to single-page allocation.
                1
            } else {
                let reason: &str = "kheap: cannot allocate uframes vec for mmap";
                error!("{reason}");
                Self::rollback_mmap(mm, vmem, vaddr, current_vaddr);
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            };
            if let Err(e) = mm.alloc_upages(vmem, current_vaddr, access, true, batch, &mut uframes)
            {
                Self::rollback_mmap(mm, vmem, vaddr, current_vaddr);
                return Err(e);
            }
            current_vaddr =
                PageAligned::from_raw_value(current_vaddr.into_raw_value() + batch * PAGE_SIZE)?;
            remaining -= batch;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Rolls back a partial `mmap` by unmapping all pages in the range
    /// `[base_vaddr, failed_vaddr)`. Best-effort: individual unmap failures are logged
    /// but do not prevent the remaining pages from being cleaned up.
    ///
    /// # Parameters
    ///
    /// - `mm`: Virtual memory manager.
    /// - `vmem`: Virtual memory space containing the mappings.
    /// - `base_vaddr`: Starting virtual address of the region to roll back.
    /// - `failed_vaddr`: Virtual address where the allocation failed (exclusive upper bound).
    ///
    fn rollback_mmap(
        mm: &mut VirtMemoryManager,
        vmem: &mut Vmem,
        base_vaddr: PageAligned<VirtualAddress>,
        failed_vaddr: PageAligned<VirtualAddress>,
    ) {
        let mut raw: usize = base_vaddr.into_raw_value();
        let end: usize = failed_vaddr.into_raw_value();
        while raw < end {
            match PageAligned::from_raw_value(raw) {
                Ok(addr) => {
                    if let Err(e) = mm.try_unmap_upage(vmem, addr) {
                        warn!("mmap rollback: failed to unmap page (vaddr={raw:#x}, error={e:?})");
                    }
                },
                Err(_) => {
                    warn!("mmap rollback: invalid page address (vaddr={raw:#x})");
                },
            }
            raw += PAGE_SIZE;
        }
    }

    pub fn munmap(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        vaddr: PageAligned<VirtualAddress>,
    ) -> Result<(), Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        let vmem: &mut Vmem = process.state_mut().vmem_mut();
        if mm.try_unmap_upage(vmem, vaddr)? {
            Ok(())
        } else {
            let reason: &str = "page is not mapped";
            error!("munmap(): {reason} (pid={pid:?}, vaddr={vaddr:?})");
            Err(Error::new(ErrorCode::NoSuchEntry, reason))
        }
    }

    pub fn mctrl(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        vaddr: PageAligned<VirtualAddress>,
        access: AccessPermission,
    ) -> Result<(), Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        let vmem: &mut Vmem = process.state_mut().vmem_mut();
        mm.ctrl_upage(vmem, vaddr, access)
    }

    pub fn mmio_alloc(
        &mut self,
        pid: ProcessIdentifier,
        region: IoMemoryRegion,
    ) -> Result<(), Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        let state: &mut ProcessState = process.state_mut();

        // Map all pages in the MMIO region.
        let vmem: &mut Vmem = state.vmem_mut();
        let base: usize = region.base().into_raw_value();
        let end: usize = base.checked_add(region.size()).ok_or_else(|| {
            let reason: &str = "mmio region end address overflow";
            error!("{reason} (base={base:#x}, size={:#?})", region.size());
            Error::new(ErrorCode::ValueOverflow, reason)
        })?;
        let perm: AccessPermission = region.perm();

        if cfg!(feature = "nightly-performance-optimizations") {
            // Single pass: apply permission changes directly.
            for raw_vaddr in (base..end).step_by(PAGE_SIZE) {
                let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?;
                vmem.kctrl(vaddr, perm, false)?;
            }
        } else {
            // Two-pass: validate that every page can be mapped before modifying any state.
            for raw_vaddr in (base..end).step_by(PAGE_SIZE) {
                let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?;
                vmem.kctrl(vaddr, perm, true)?;
            }

            // All validations passed — apply the permission changes for real.
            for raw_vaddr in (base..end).step_by(PAGE_SIZE) {
                let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(raw_vaddr)?;
                vmem.kctrl(vaddr, perm, false)?;
            }
        }

        state.add_mmio(region);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Detaches a memory-mapped I/O region identified by `tag` from the process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process from which to detach the region.
    /// - `tag`: Tag that uniquely identifies the MMIO region to detach.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Upon failure, an error is returned instead.
    ///
    pub fn mmio_free(&mut self, pid: ProcessIdentifier, tag: MmioTag) -> Result<(), Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        let state: &mut ProcessState = process.state_mut();
        state.remove_mmio(tag);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Retrieves metadata for the MMIO region identified by `tag` attached to a process.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process that owns the region.
    /// - `tag`: Tag that uniquely identifies the MMIO region.
    ///
    /// # Returns
    ///
    /// Upon success, a tuple containing the base address, size, and access permissions of the
    /// region is returned. Upon failure, an error is returned instead.
    ///
    pub fn mmio_info(
        &self,
        pid: ProcessIdentifier,
        tag: MmioTag,
    ) -> Result<(PageAligned<VirtualAddress>, usize, AccessPermission), Error> {
        let process: ProcessRef = self.find_process(pid)?;
        let state: &ProcessState = process.state();

        match state.mmio_info(tag) {
            Some(region) => Ok((region.base(), region.size(), region.perm())),
            None => {
                let reason: &'static str = "mmio region not found";
                error!("{reason}");
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }

    pub fn attach_pmio(&mut self, pid: ProcessIdentifier, port: AnyIoPort) -> Result<(), Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        process.state_mut().add_pmio(port);
        Ok(())
    }

    pub fn detach_pmio(
        &mut self,
        pid: ProcessIdentifier,
        port_number: u16,
    ) -> Result<AnyIoPort, Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        process.state_mut().remove_pmio(port_number)
    }

    pub fn read_pmio(
        &mut self,
        pid: ProcessIdentifier,
        port_number: u16,
        port_width: IoPortWidth,
    ) -> Result<u32, Error> {
        let process: ProcessRef = self.find_process(pid)?;
        process.state().read_pmio(port_number, port_width)
    }

    pub fn write_pmio(
        &mut self,
        pid: ProcessIdentifier,
        port_number: u16,
        port_width: IoPortWidth,
        value: u32,
    ) -> Result<(), Error> {
        let mut process: ProcessRefMut = self.find_process_mut(pid)?;
        process
            .state_mut()
            .write_pmio(port_number, port_width, value)
    }

    ///
    /// # Description
    ///
    /// Posts a message.
    ///
    /// # Parameters
    ///
    /// - `receiver`: ID of the receiver.
    /// - `message`: Message to send.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    pub fn post_message(
        &mut self,
        receiver: MessageReceiver,
        message: Message,
    ) -> Result<(), Error> {
        {
            let mut process: ProcessRefMut = match receiver.as_id() {
                Ok(pid) => self.find_process_mut(pid)?,
                Err(tid) => self.find_process_by_tid(tid)?,
            };
            process.state_mut().post_message(message);
        }
        self.note_message_posted()?;
        Ok(())
    }

    pub fn add_event(&mut self, ownership: EventOwnership) -> Result<(), Error> {
        self.get_running_mut().state_mut().add_event(ownership);

        Ok(())
    }

    pub fn remove_event(&mut self, ev: &Event) -> Result<(), Error> {
        self.get_running_mut().state_mut().remove_event(ev);

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Handles a page fault that may be caused by user-space stack growth.
    /// If the faulting address falls within the user stack region and the page is not present,
    /// a new page is demand-allocated and mapped.
    ///
    /// # Parameters
    ///
    /// - `mm`: Virtual memory manager.
    /// - `fault_addr`: Faulting virtual address.
    /// - `error_code`: Typed x86 page-fault error code.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the fault was resolved by mapping a new stack page.
    /// - `Ok(false)` if the fault address is not within the user stack region or the fault is not
    ///   a page-not-present fault from user mode.
    /// - `Err(...)` if page allocation failed.
    ///
    pub fn handle_stack_page_fault(
        &mut self,
        mm: &mut VirtMemoryManager,
        fault_addr: usize,
        error_code: excp::ErrorCode,
    ) -> Result<bool, Error> {
        // Only handle page-not-present faults from user mode.
        if error_code.is_present() || !error_code.is_user() {
            return Ok(false);
        }

        // Check if the faulting address falls within the main-thread user stack region.
        // The stack occupies [USER_STACK_TOP_RAW, USER_STACK_TOP_RAW + USER_STACK_SIZE).
        const STACK_REGION_START: usize = USER_STACK_TOP_RAW;
        const STACK_REGION_END: usize = STACK_REGION_START + USER_STACK_SIZE;
        if !(STACK_REGION_START..STACK_REGION_END).contains(&fault_addr) {
            return Ok(false);
        }

        // Page-align the faulting address and map the page.
        let page_addr: usize = fault_addr & !(PAGE_SIZE - 1);
        let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(page_addr)?;
        let pid: ProcessIdentifier = self.get_pid();
        debug!("demand-paging stack page (pid={pid:?}, vaddr={vaddr:?})");
        self.mmap(mm, pid, vaddr, 1, AccessPermission::RDWR)?;

        Ok(true)
    }
}
