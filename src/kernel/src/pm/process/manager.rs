// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
            PageAddress,
            PageAligned,
            VirtualAddress,
        },
    },
    mm::{
        elf::Elf32Fhdr,
        ustack::{
            UserStack,
            UserStackAllocator,
        },
        KernelPage,
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        process::{
            identity::ProcessIdentity,
            state::{
                InterruptedProcess,
                ProcessRef,
                ProcessRefMut,
                ProcessState,
                RunnableProcess,
                RunningProcess,
                SleepingProcess,
                ZombieProcess,
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
    rc::Rc,
    vec::Vec,
};
use ::core::cell::{
    Ref,
    RefCell,
    RefMut,
};
use ::sys::{
    arch::mem::PAGE_SIZE,
    error::{
        Error,
        ErrorCode,
    },
    event::Event,
    ipc::Message,
    pm::{
        Capability,
        GroupIdentifier,
        ProcessIdentifier,
        ThreadIdentifier,
        UserIdentifier,
    },
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
        let kernel: RunnableProcess = RunnableProcess::new(
            ProcessIdentifier::KERNEL,
            ProcessIdentity::new(UserIdentifier::ROOT, GroupIdentifier::ROOT),
            kernel,
            root,
            None,
        );

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
        user_func: VirtualAddress,
        enable_interrupts: bool,
    ) -> Result<ContextInformation, Error> {
        trace!(
            "forge_user_context(): user_stack={:?}, user_func={:?}, enable_interrupts={}",
            user_stack,
            user_func,
            enable_interrupts
        );

        extern "C" {
            pub fn __leave_kernel_to_user_mode();
        }

        // Ensure that user function lies within the user address space.
        if !Vmem::is_user_addr(user_func) {
            let reason: &str = "user function is not within the user address space";
            error!(
                "forge_context(): {} (user_stack={:?}, user_func={:?})",
                reason, user_stack, user_func
            );
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let kernel_func: VirtualAddress =
            VirtualAddress::from_raw_value(__leave_kernel_to_user_mode as usize);

        // Alloc kernel pages for the kernel stack.
        // NOTE: if we fail, kernel pages allocated for the kernel stack are deallocated.
        let mut kpages: Vec<KernelPage> =
            mm.alloc_kpages(true, config::kernel::KSTACK_SIZE / PAGE_SIZE)?;
        let base: PageAddress = kpages[0].base();
        let kernel_stack: usize =
            unsafe { (base.into_raw_value() as *mut u8).add(config::kernel::KSTACK_SIZE) } as usize;

        let cr3: u32 = vmem.pgdir().physical_address()?.into_raw_value() as u32;
        let esp: u32 = unsafe {
            hal::arch::forge_user_stack(
                kernel_stack as *mut u8,
                user_stack.top().into_raw_value(),
                user_func.into_raw_value(),
                kernel_func.into_raw_value(),
                enable_interrupts,
            )
        } as u32;
        let esp0: u32 = kernel_stack as u32;

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

        // Map kernel stack.
        while let Some(kpage) = kpages.pop() {
            vmem.add_private_kernel_page(kpage);
        }

        Ok(context)
    }

    ///
    /// # Description
    ///
    /// Creates a new process.
    ///
    /// # Parameters
    ///
    /// - `mm`: Memory manager to use.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the process identifier of the new process is returned.
    /// Otherwise, an error is returned instead.
    ///
    fn create_process(&mut self, mm: &mut VirtMemoryManager) -> Result<ProcessIdentifier, Error> {
        extern "C" {
            pub fn __leave_kernel_to_user_mode();
        }

        trace!("create_process()");

        // Create a new memory address space for the process.
        let mut vmem: Vmem = mm.new_vmem(self.get_running().state().vmem())?;

        // Create a stack allocator.
        let user_stack_allocator: UserStackAllocator = UserStackAllocator::new()?;

        // Create a kernel context.
        let user_stack: UserStack = user_stack_allocator.alloc()?;
        let user_func: VirtualAddress = ::sys::config::memory_layout::USER_BASE;
        let context: ContextInformation = Self::forge_user_context(
            mm,
            &mut vmem,
            &user_stack,
            user_func,
            self.interrupt_capable,
        )?;

        //==============================================================
        // NOTE: if we fail beyond this point we need to page mappings.
        //==============================================================

        let thread: ReadyThread = self.tm.create_thread(Some(user_stack), context);

        // Create process.
        let pid: ProcessIdentifier = self.next_pid;
        self.next_pid = ProcessIdentifier::from(Into::<u32>::into(pid) + 1);
        let identity: ProcessIdentity = self.get_running().state().identity().clone();
        let process: RunnableProcess =
            RunnableProcess::new(pid, identity, thread, vmem, Some(user_stack_allocator));

        // Add process to the queue of ready processes.
        self.ready.push_back(process);

        Ok(pid)
    }

    /// Schedule a process to run.
    pub fn schedule(&mut self) -> (*mut ContextInformation, *mut ContextInformation) {
        // Reschedule running process.
        let previous_process: RunningProcess = self.take_running();
        let (previous_process, previous_context) = previous_process.schedule();
        self.ready.push_back(previous_process);

        // Select next ready process to run.
        if let Some(next_process) = self.interrupted.pop_back() {
            let (next_process, reason, next_context): (
                RunningProcess,
                InterruptReason,
                *mut ContextInformation,
            ) = next_process.resume();
            self.interrupt_reason = Some(reason);
            self.running = Some(next_process);
            (previous_context, next_context)
        } else {
            let next_process: RunnableProcess = self.take_ready();
            let (next_process, reason, next_context): (
                RunningProcess,
                Option<InterruptReason>,
                *mut ContextInformation,
            ) = next_process.run();

            self.interrupt_reason = reason;
            self.running = Some(next_process);
            (previous_context, next_context)
        }
    }

    pub fn exec(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        elf: &Elf32Fhdr,
    ) -> Result<(), Error> {
        // Find corresponding process.
        let process: &mut RunnableProcess =
            match self.ready.iter_mut().find(|p| p.state().pid() == pid) {
                Some(p) => p,
                None => {
                    let reason: &str = "process not found";
                    error!("exec(): {}", reason);
                    return Err(Error::new(ErrorCode::NoSuchProcess, reason));
                },
            };

        process.exec(mm, elf)?;

        Ok(())
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
    fn sleep(&mut self) -> (*mut ContextInformation, *mut ContextInformation) {
        let running_process: RunningProcess = self.take_running();

        // Check if kernel is trying to sleep.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot sleep");
        }

        match running_process.sleep() {
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
    /// - `tid`: ID of the thread to wake up.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error code is returned instead.
    ///
    pub fn wakeup(&mut self, tid: ThreadIdentifier) -> Result<(), Error> {
        let runnable_process: RunnableProcess = match self.try_wakeup(tid) {
            Some(runnable_process) => runnable_process,
            None => {
                let reason: &str = "thread not found";
                error!("wake_up(): {}", reason);
                return Err(Error::new(ErrorCode::NoSuchEntry, reason));
            },
        };

        self.ready.push_back(runnable_process);

        Ok(())
    }

    fn try_wakeup(&mut self, tid: ThreadIdentifier) -> Option<RunnableProcess> {
        let mut suspended: LinkedList<SleepingProcess> = LinkedList::new();
        while let Some(process) = self.suspended.pop_front() {
            match process.wakeup(tid) {
                Ok(runnable_process) => {
                    while let Some(process) = suspended.pop_front() {
                        self.suspended.push_back(process);
                    }
                    return Some(runnable_process);
                },
                Err(suspended_process) => suspended.push_back(suspended_process),
            }
        }
        while let Some(process) = suspended.pop_front() {
            self.suspended.push_back(process);
        }

        let mut ready: LinkedList<RunnableProcess> = LinkedList::new();
        while let Some(process) = self.ready.pop_front() {
            match process.wakeup(tid) {
                Ok(runnable_process) => {
                    while let Some(process) = ready.pop_front() {
                        self.ready.push_back(process);
                    }
                    return Some(runnable_process);
                },
                Err(ready_process) => ready.push_back(ready_process),
            }
        }
        while let Some(process) = ready.pop_front() {
            self.ready.push_back(process);
        }

        None
    }

    pub fn exit(&mut self, status: i32) -> (*mut ContextInformation, *mut ContextInformation) {
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

    pub fn harvest_zombies(
        &mut self,
        mm: &mut VirtMemoryManager,
    ) -> Option<(ProcessIdentifier, i32)> {
        if let Some(zombie) = self.zombies.pop_front() {
            let (zombie_threads, mut state, status): (
                NonEmptyVecDeque<ZombieThread>,
                Box<ProcessState>,
                i32,
            ) = zombie.bury();
            let (mut more_zombie_threads, zombie_thread): (VecDeque<ZombieThread>, ZombieThread) =
                zombie_threads.pop_front();
            more_zombie_threads.push_front(zombie_thread);

            // Traverse the list of zombie threads.
            while let Some(zombie_thread) = more_zombie_threads.pop_front() {
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
                        if let Err(error) = mm.unmap_upage(state.vmem_mut(), vaddr) {
                            // We failed, but this is not too bad, as we will free all pages
                            // when wiping out the address space anyways.
                            warn!(
                                "harvest_zombies(): failed to unmap page (vaddr={:?}, error={:?})",
                                vaddr, error
                            );
                        }
                    }
                }
            }

            Some((state.pid(), status))
        } else {
            None
        }
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

static mut PROCESS_MANAGER: Option<ProcessManager> = None;

impl ProcessManager {
    /// Creates a new process.
    pub fn create_process(
        &mut self,
        mm: &mut VirtMemoryManager,
    ) -> Result<ProcessIdentifier, Error> {
        self.try_borrow_mut()?.create_process(mm)
    }

    pub fn exec(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        elf: &Elf32Fhdr,
    ) -> Result<(), Error> {
        self.try_borrow_mut()?.exec(mm, pid, elf)
    }

    pub fn getuid(&self, pid: ProcessIdentifier) -> Result<UserIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_uid())
    }

    pub fn setuid(&mut self, pid: ProcessIdentifier, uid: UserIdentifier) -> Result<(), Error> {
        self.try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_uid(uid)
    }

    pub fn geteuid(&self, pid: ProcessIdentifier) -> Result<UserIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_euid())
    }

    pub fn seteuid(&mut self, pid: ProcessIdentifier, euid: UserIdentifier) -> Result<(), Error> {
        self.try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_euid(euid)
    }

    pub fn getgid(&self, pid: ProcessIdentifier) -> Result<GroupIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_gid())
    }

    pub fn setgid(&mut self, pid: ProcessIdentifier, gid: GroupIdentifier) -> Result<(), Error> {
        self.try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_gid(gid)
    }

    pub fn getegid(&self, pid: ProcessIdentifier) -> Result<GroupIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_egid())
    }

    pub fn setegid(&mut self, pid: ProcessIdentifier, egid: GroupIdentifier) -> Result<(), Error> {
        self.try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_egid(egid)
    }

    pub fn capctl(
        &mut self,
        pid: ProcessIdentifier,
        capability: Capability,
        value: bool,
    ) -> Result<(), Error> {
        self.try_borrow_mut()?.capctl(pid, capability, value)
    }

    pub fn has_capability(pid: ProcessIdentifier, capability: Capability) -> Result<bool, Error> {
        Ok(Self::get()?
            .try_borrow()?
            .find_process(pid)?
            .state()
            .has_capability(capability))
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
        let (from, to): (*mut ContextInformation, *mut ContextInformation) =
            Self::get_mut()?.try_borrow_mut()?.exit(status);

        ContextInformation::switch(from, to);
        core::hint::unreachable_unchecked()
    }

    pub fn terminate(&mut self, pid: ProcessIdentifier) -> Result<(), Error> {
        self.try_borrow_mut()?.terminate(pid)
    }

    pub fn vmcopy_from_user(
        pid: ProcessIdentifier,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        Self::get_mut()?
            .try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .copy_from_user_unaligned(dst, src, size)
    }

    pub fn vmcopy_to_user(
        pid: ProcessIdentifier,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        Self::get_mut()?
            .try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .copy_to_user_unaligned(dst, src, size)
    }

    pub fn harvest_zombies(
        &mut self,
        mm: &mut VirtMemoryManager,
    ) -> Result<Option<(ProcessIdentifier, i32)>, Error> {
        Ok(self.try_borrow_mut()?.harvest_zombies(mm))
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
    /// Returns the ID of the calling process.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the ID of the calling process is returned. Otherwise, an error
    /// code is returned instead.
    ///
    pub fn get_pid() -> Result<ProcessIdentifier, Error> {
        Ok(Self::get()?.try_borrow()?.get_running().state().pid())
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
    pub fn get_tid() -> Result<ThreadIdentifier, Error> {
        Ok(Self::get()?.try_borrow()?.get_running().get_tid())
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
            .map_err(SleepError::Generic)?
            .try_borrow_mut()
            .map_err(SleepError::Generic)?
            .sleep();

        ContextInformation::switch(from, to);

        let interrupt_reason: Option<InterruptReason> = Self::get_mut()
            .map_err(SleepError::Generic)?
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
    pub fn wakeup(tid: ThreadIdentifier) -> Result<(), Error> {
        Self::get_mut()?.try_borrow_mut()?.wakeup(tid)
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
            { Self::get_mut()?.try_borrow_mut()?.schedule() };

        ContextInformation::switch(from, to);

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
    pub fn try_recv() -> Result<Option<Message>, Error> {
        let mut pm: RefMut<ProcessManagerInner> = Self::get_mut()?.try_borrow_mut()?;
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

    fn get() -> Result<&'static ProcessManager, Error> {
        unsafe {
            match PROCESS_MANAGER {
                Some(ref pm) => Ok(pm),
                None => {
                    let reason: &str = "process manager not initialized";
                    error!("get(): {}", reason);
                    Err(Error::new(ErrorCode::TryAgain, reason))
                },
            }
        }
    }

    fn get_mut() -> Result<&'static mut ProcessManager, Error> {
        unsafe {
            match PROCESS_MANAGER {
                Some(ref mut pm) => Ok(pm),
                None => {
                    let reason: &str = "process manager not initialized";
                    error!("get_mut(): {}", reason);
                    Err(Error::new(ErrorCode::TryAgain, reason))
                },
            }
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Initializes the process manager.
pub fn init(
    interrupt_capable: bool,
    kernel: ReadyThread,
    root: Vmem,
    tm: ThreadManager,
) -> ProcessManager {
    // TODO: check for double initialization.

    let pm: Rc<RefCell<ProcessManagerInner>> =
        Rc::new(RefCell::new(ProcessManagerInner::new(interrupt_capable, kernel, root, tm)));

    unsafe { PROCESS_MANAGER = Some(ProcessManager(pm.clone())) };

    ProcessManager(pm)
}
