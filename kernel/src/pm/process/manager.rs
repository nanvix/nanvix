// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
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
        self,
        elf::Elf32Fhdr,
        KernelPage,
        VirtMemoryManager,
        Vmem,
    },
    pm::{
        process::{
            identity::ProcessIdentity,
            process::{
                InterruptReason,
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
            ReadyThread,
            ThreadManager,
        },
    },
};
use ::alloc::{
    collections::LinkedList,
    rc::Rc,
    vec::Vec,
};
use ::core::cell::{
    Ref,
    RefCell,
    RefMut,
};
use ::sys::{
    config,
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

//==================================================================================================
// Process Manager Inner
//==================================================================================================

///
/// # Description
///
/// A type that represents the process manager.
///
struct ProcessManagerInner {
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
}

impl ProcessManagerInner {
    /// Initializes the process manager.
    pub fn new(kernel: ReadyThread, root: Vmem, tm: ThreadManager) -> Self {
        let kernel: RunnableProcess = RunnableProcess::new(
            ProcessIdentifier::from(0),
            ProcessIdentity::new(UserIdentifier::ROOT, GroupIdentifier::ROOT),
            kernel,
            root,
        );

        let (kernel, _): (RunningProcess, *mut ContextInformation) = kernel.run();

        Self {
            interrupt_reason: None,
            next_pid: ProcessIdentifier::from(1),
            ready: LinkedList::new(),
            suspended: LinkedList::new(),
            interrupted: LinkedList::new(),
            zombies: LinkedList::new(),
            running: Some(kernel),
            tm,
        }
    }

    fn forge_user_context(
        vmem: &Vmem,
        user_stack: VirtualAddress,
        user_func: VirtualAddress,
        kernel_func: VirtualAddress,
        kernel_stack: VirtualAddress,
    ) -> Result<ContextInformation, Error> {
        let cr3: u32 = vmem.pgdir().physical_address()?.into_raw_value() as u32;
        let esp: u32 = unsafe {
            hal::arch::forge_user_stack(
                kernel_stack.into_raw_value() as *mut u8,
                user_stack.into_raw_value(),
                user_func.into_raw_value(),
                kernel_func.into_raw_value(),
            )
        } as u32;
        let esp0: u32 = kernel_stack.into_raw_value() as u32;

        trace!("forge_context(): cr3={:#x}, esp={:#x}, ebp={:#x}", cr3, esp, esp0);
        Ok(ContextInformation::new(cr3, esp, esp0))
    }

    /// Creates a new thread.
    pub fn create_thread(
        &mut self,
        mm: &mut VirtMemoryManager,
        vmem: &mut Vmem,
        user_stack: VirtualAddress,
        user_func: VirtualAddress,
        kernel_func: VirtualAddress,
    ) -> Result<ReadyThread, Error> {
        let mut kpages: Vec<KernelPage> =
            mm.alloc_kpages(true, config::kernel::KSTACK_SIZE / mem::PAGE_SIZE)?;

        let base: PageAddress = kpages[0].base();
        let size: usize = config::kernel::KSTACK_SIZE;
        let top = unsafe { (base.into_raw_value() as *mut u8).add(size) };
        let kernel_stack = VirtualAddress::from_raw_value(top as usize)?;

        let context: ContextInformation =
            Self::forge_user_context(vmem, user_stack, user_func, kernel_func, kernel_stack)?;

        while let Some(kpage) = kpages.pop() {
            vmem.add_private_kernel_page(kpage);
        }

        self.tm.create_thread(context)
    }

    /// Creates a new process.
    pub fn create_process(
        &mut self,
        mm: &mut VirtMemoryManager,
    ) -> Result<ProcessIdentifier, Error> {
        extern "C" {
            pub fn __leave_kernel_to_user_mode();
        }

        trace!("create_process()");

        // Create a new memory address space for the process.
        let mut vmem: Vmem = mm.new_vmem(self.get_running().state().vmem())?;

        let user_stack: VirtualAddress = mm::user_stack_top().into_inner();
        let user_func: VirtualAddress = ::sys::config::memory_layout::USER_BASE;
        let kernel_func: VirtualAddress =
            VirtualAddress::from_raw_value(__leave_kernel_to_user_mode as usize)?;
        let thread: ReadyThread =
            self.create_thread(mm, &mut vmem, user_stack, user_func, kernel_func)?;

        // Alloc user stack.
        let vaddr: PageAligned<VirtualAddress> = PageAligned::from_raw_value(
            mm::user_stack_top().into_raw_value() - config::kernel::KSTACK_SIZE - mem::PAGE_SIZE,
        )?;
        mm.alloc_upages(
            &mut vmem,
            vaddr,
            config::kernel::KSTACK_SIZE / mem::PAGE_SIZE,
            AccessPermission::RDWR,
        )?;

        let pid: ProcessIdentifier = self.next_pid;
        self.next_pid = ProcessIdentifier::from(Into::<usize>::into(pid) + 1);
        let identity: ProcessIdentity = self.get_running().state().identity().clone();
        let process: RunnableProcess = RunnableProcess::new(pid, identity, thread, vmem);

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
            let (next_process, reason, next_context) = next_process.resume();
            self.interrupt_reason = Some(reason);
            self.running = Some(next_process);
            return (previous_context, next_context);
        } else {
            let next_process: RunnableProcess = self.take_ready();
            let (next_process, next_context): (RunningProcess, *mut ContextInformation) =
                next_process.run();

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
    pub fn sleep(&mut self) -> (*mut ContextInformation, *mut ContextInformation) {
        let running_process: RunningProcess = self.take_running();

        // Check if kernel is trying to sleep.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot sleep");
        }

        match running_process.sleep() {
            Ok((runnable_process, previous_context)) => {
                let (next_process, next_context) = runnable_process.run();
                self.running = Some(next_process);
                (previous_context, next_context)
            },
            Err((suspended_process, previous_context)) => {
                self.suspended.push_back(suspended_process);
                let next_process: RunnableProcess = self.take_ready();
                let (next_process, next_context) = next_process.run();
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
    ///GroupIdentifier
    pub fn wakeup(&mut self, tid: ThreadIdentifier) -> Result<(), Error> {
        let mut suspended: LinkedList<SleepingProcess> = LinkedList::new();
        while let Some(process) = self.suspended.pop_front() {
            match process.wakeup_sleeping_thread(tid) {
                Ok(runnable_process) => {
                    self.ready.push_back(runnable_process);
                    while let Some(process) = suspended.pop_front() {
                        self.suspended.push_back(process);
                    }
                    return Ok(());
                },
                Err(suspended_process) => suspended.push_back(suspended_process),
            }
        }

        let reason: &str = "thread not found";
        error!("wake_up(): {}", reason);
        Err(Error::new(ErrorCode::NoSuchEntry, reason))
    }

    pub fn exit(
        &mut self,
        status: i32,
    ) -> Result<(*mut ContextInformation, *mut ContextInformation), Error> {
        let running_process: RunningProcess = self.take_running();

        // Check if kernel is trying to exit.
        if running_process.state().pid() == ProcessIdentifier::KERNEL {
            panic!("kernel process cannot exit");
        }

        match running_process.exit(status) {
            Ok((runnable_process, previous_context)) => {
                let (running_process, next_context) = runnable_process.run();
                self.running = Some(running_process);
                Ok((previous_context, next_context))
            },
            Err((zombie_process, previous_context)) => {
                self.zombies.push_back(zombie_process);

                match self.ready.pop_front() {
                    Some(runnable_process) => {
                        let (running_process, next_context) = runnable_process.run();
                        self.running = Some(running_process);
                        Ok((previous_context, next_context))
                    },
                    None => unreachable!("there should be a process ready to run"),
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
            let process: ZombieProcess = process.terminate();
            self.zombies.push_back(process);
            return Ok(());
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

    pub fn harvest_zombies(&mut self) {
        while let Some(mut zombie) = self.zombies.pop_front() {
            let (thread, state, status) = zombie.bury();
            trace!(
                "haversint resources (pid={:?}, tid={:?}, status={:?})",
                state.pid(),
                thread.id(),
                status
            );
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
            error!("find_process(): {}", reason);
            Err(Error::new(ErrorCode::NoSuchProcess, reason))
        }
    }

    fn find_process_mut(&mut self, pid: ProcessIdentifier) -> Result<ProcessRefMut, Error> {
        if self.get_running_mut().state_mut().pid() == pid {
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
            error!("find_process(): {}", reason);
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
        Ok(self.try_borrow_mut()?.create_process(mm)?)
    }

    pub fn exec(
        &mut self,
        mm: &mut VirtMemoryManager,
        pid: ProcessIdentifier,
        elf: &Elf32Fhdr,
    ) -> Result<(), Error> {
        Ok(self.try_borrow_mut()?.exec(mm, pid, elf)?)
    }

    pub fn getuid(&self, pid: ProcessIdentifier) -> Result<UserIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_uid())
    }

    pub fn setuid(&mut self, pid: ProcessIdentifier, uid: UserIdentifier) -> Result<(), Error> {
        Ok(self
            .try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_uid(uid)?)
    }

    pub fn geteuid(&self, pid: ProcessIdentifier) -> Result<UserIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_euid())
    }

    pub fn seteuid(&mut self, pid: ProcessIdentifier, euid: UserIdentifier) -> Result<(), Error> {
        Ok(self
            .try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_euid(euid)?)
    }

    pub fn getgid(&self, pid: ProcessIdentifier) -> Result<GroupIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_gid())
    }

    pub fn setgid(&mut self, pid: ProcessIdentifier, gid: GroupIdentifier) -> Result<(), Error> {
        Ok(self
            .try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_gid(gid)?)
    }

    pub fn getegid(&self, pid: ProcessIdentifier) -> Result<GroupIdentifier, Error> {
        Ok(self.try_borrow()?.find_process(pid)?.state().get_egid())
    }

    pub fn setegid(&mut self, pid: ProcessIdentifier, egid: GroupIdentifier) -> Result<(), Error> {
        Ok(self
            .try_borrow_mut()?
            .find_process_mut(pid)?
            .state_mut()
            .set_egid(egid)?)
    }

    pub fn capctl(
        &mut self,
        pid: ProcessIdentifier,
        capability: Capability,
        value: bool,
    ) -> Result<(), Error> {
        Ok(self.try_borrow_mut()?.capctl(pid, capability, value)?)
    }

    pub fn has_capability(pid: ProcessIdentifier, capability: Capability) -> Result<bool, Error> {
        Ok(Self::get()?
            .try_borrow()?
            .find_process(pid)?
            .state()
            .has_capability(capability))
    }

    pub fn exit(status: i32) -> Result<!, Error> {
        trace!("exit({:?})", status);
        let (from, to): (*mut ContextInformation, *mut ContextInformation) =
            Self::get_mut()?.try_borrow_mut()?.exit(status)?;

        unsafe {
            ContextInformation::switch(from, to);
            core::hint::unreachable_unchecked()
        }
    }

    pub fn terminate(&mut self, pid: ProcessIdentifier) -> Result<(), Error> {
        Ok(self.try_borrow_mut()?.terminate(pid)?)
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

    pub fn harvest_zombies(&mut self) -> Result<(), Error> {
        self.try_borrow_mut()?.harvest_zombies();
        Ok(())
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
        mm.alloc_upage(vmem, vaddr, access)
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
    pub fn sleep() -> Result<(), Error> {
        let (from, to): (*mut ContextInformation, *mut ContextInformation) =
            Self::get_mut()?.try_borrow_mut()?.sleep();

        unsafe { ContextInformation::switch(from, to) }

        let interrupt_reason: Option<InterruptReason> =
            Self::get_mut()?.try_borrow_mut()?.interrupt_reason();

        if interrupt_reason.is_some() {
            let reason: &str = "interrupted";
            error!("sleep(): {}", reason);
            return Err(Error::new(ErrorCode::Interrupted, reason));
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

    pub fn switch() -> Result<(), Error> {
        let (from, to): (*mut ContextInformation, *mut ContextInformation) =
            { Self::get_mut()?.try_borrow_mut()?.schedule() };

        unsafe { ContextInformation::switch(from, to) }

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

    pub fn post_message(&mut self, pid: ProcessIdentifier, message: Message) -> Result<(), Error> {
        let mut pm: RefMut<ProcessManagerInner> = self.try_borrow_mut()?;
        let mut process: ProcessRefMut = pm.find_process_mut(pid)?;
        process.state_mut().post_message(message);

        Ok(())
    }

    pub fn try_recv() -> Result<Option<Message>, Error> {
        Ok(Self::get_mut()?
            .try_borrow_mut()?
            .get_running_mut()
            .state_mut()
            .receive_message())
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
pub fn init(kernel: ReadyThread, root: Vmem, tm: ThreadManager) -> ProcessManager {
    // TODO: check for double initialization.

    let pm: Rc<RefCell<ProcessManagerInner>> =
        Rc::new(RefCell::new(ProcessManagerInner::new(kernel, root, tm)));

    unsafe { PROCESS_MANAGER = Some(ProcessManager(pm.clone())) };

    ProcessManager(pm)
}
