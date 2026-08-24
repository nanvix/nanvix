// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "test")]
mod delivery_test;
mod interrupted;
#[cfg(feature = "test")]
mod kill_test;
mod runnable;
mod running;
pub(crate) mod sigframe;
pub(crate) mod signal;
#[cfg(feature = "test")]
mod signal_test;
mod sleeping;
#[cfg(feature = "test")]
mod test_detach;
mod zombie;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventOwnership,
    hal::{
        io::{
            AnyIoPort,
            IoMemoryRegion,
            IoPortWidth,
            MmioTag,
        },
        mem::VirtualAddress,
    },
    ipc::Mailbox,
    mm::Vmem,
    pm::{
        process::{
            capability::Capabilities,
            state::signal::SignalControl,
            LifecycleTerminationCredit,
        },
        sync::{
            condvar::Condvar,
            mutex::Mutex,
        },
        thread::{
            ThreadRef,
            ThreadRefMut,
        },
        DeliverySequence,
    },
};
use ::alloc::collections::{
    btree_map::BTreeMap,
    LinkedList,
};
use ::config::kernel::{
    COND_OPEN_MAX,
    MUTEX_OPEN_MAX,
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
    ExitStatus,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use interrupted::InterruptedProcess;
pub use runnable::RunnableProcess;
pub use running::RunningProcess;
pub use signal::exception_to_signal;
pub use sleeping::SleepingProcess;
pub use zombie::ZombieProcess;

///
/// # Description
///
/// Reports whether an ownership guard matches the event being unregistered. Scheduling events are
/// owned as a single class, so any scheduling guard matches a scheduling unregistration; other
/// events match exactly.
///
/// # Parameters
///
/// - `owned`: Event carried by the ownership guard.
/// - `requested`: Event being unregistered.
///
/// # Returns
///
/// `true` if the guard matches the requested event, otherwise `false`.
///
pub(crate) fn event_guard_matches(owned: &Event, requested: &Event) -> bool {
    match requested {
        Event::Scheduling(_) => matches!(owned, Event::Scheduling(_)),
        _ => owned == requested,
    }
}

//==================================================================================================
// Tests
//==================================================================================================

/// Runs all in-kernel unit tests for the process state module.
#[cfg(feature = "test")]
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= delivery_test::test();
    passed &= test_detach::test();
    passed &= signal_test::test();
    passed &= sigframe::test();
    passed &= kill_test::test();
    passed
}

//==================================================================================================
// ProcessRefMut
//==================================================================================================

pub enum ProcessRefMut<'a> {
    Runnable(&'a mut RunnableProcess),
    Running(&'a mut RunningProcess),
    Sleeping(&'a mut SleepingProcess),
    Interrupted(&'a mut InterruptedProcess),
    Zombie(&'a mut ZombieProcess),
}

impl ProcessRefMut<'_> {
    pub fn state_mut(&mut self) -> &mut ProcessState {
        match self {
            ProcessRefMut::Runnable(process) => process.state_mut(),
            ProcessRefMut::Running(process) => process.state_mut(),
            ProcessRefMut::Sleeping(process) => process.state_mut(),
            ProcessRefMut::Interrupted(process) => process.state_mut(),
            ProcessRefMut::Zombie(process) => process.state_mut(),
        }
    }

    ///
    /// # Description
    ///
    /// Finds a mutable reference to a thread by identifier, searching across all process states.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the thread to find.
    ///
    /// # Returns
    ///
    /// If found, returns `Some` with a mutable thread reference. Otherwise, returns `None`.
    ///
    pub fn find_thread_mut(&mut self, tid: ThreadIdentifier) -> Option<ThreadRefMut<'_>> {
        match self {
            ProcessRefMut::Runnable(process) => process.find_thread_mut(tid),
            ProcessRefMut::Running(process) => process.find_thread_mut(tid),
            ProcessRefMut::Sleeping(process) => process.find_thread_mut(tid),
            ProcessRefMut::Interrupted(process) => process.find_thread_mut(tid),
            ProcessRefMut::Zombie(process) => process.find_thread_mut(tid),
        }
    }
}

pub enum ProcessRef<'a> {
    Runnable(&'a RunnableProcess),
    Running(&'a RunningProcess),
    Sleeping(&'a SleepingProcess),
    Interrupted(&'a InterruptedProcess),
    Zombie(&'a ZombieProcess),
}

impl ProcessRef<'_> {
    pub fn state(&self) -> &ProcessState {
        match self {
            ProcessRef::Runnable(process) => process.state(),
            ProcessRef::Running(process) => process.state(),
            ProcessRef::Sleeping(process) => process.state(),
            ProcessRef::Interrupted(process) => process.state(),
            ProcessRef::Zombie(process) => process.state(),
        }
    }

    ///
    /// # Description
    ///
    /// Finds an immutable reference to a thread by identifier, searching across all process states.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the thread to find.
    ///
    /// # Returns
    ///
    /// If found, returns `Some` with a thread reference. Otherwise, returns `None`.
    ///
    pub fn find_thread(&self, tid: ThreadIdentifier) -> Option<ThreadRef<'_>> {
        match self {
            ProcessRef::Runnable(process) => process.find_thread(tid),
            ProcessRef::Running(process) => process.find_thread(tid),
            ProcessRef::Sleeping(process) => process.find_thread(tid),
            ProcessRef::Interrupted(process) => process.find_thread(tid),
            ProcessRef::Zombie(process) => process.find_thread(tid),
        }
    }
}

//==================================================================================================
// Process
//==================================================================================================

///
/// # Description
///
/// A type that represents the inner state of a process.
///
pub struct ProcessState {
    /// Process identifier.
    pid: ProcessIdentifier,
    /// Process identifier of the parent process.
    parent: ProcessIdentifier,
    /// Capacity reserved for this process's future lifecycle termination record. The kernel process
    /// has no credit because it has no lifecycle creation or termination record.
    termination_credit: Option<LifecycleTerminationCredit>,
    /// Capabilities.
    capabilities: Capabilities,
    /// Memory address space.
    vmem: Vmem,
    /// Event ownerships.
    events: LinkedList<EventOwnership>,
    /// Incoming messages.
    mailbox: Mailbox,
    /// Next event-service class considered by this process.
    delivery_cursor: usize,
    /// Memory mapped I/O regions.
    mmio: LinkedList<IoMemoryRegion>,
    /// I/O ports.
    pmio: LinkedList<AnyIoPort>,
    /// Mutexes.
    mutexes: BTreeMap<MutexAddress, Mutex>,
    /// Condition variables.
    conditions: BTreeMap<ConditionAddress, Condvar>,
    /// Pending exit status set when `exit()` is called with threads still running.
    pending_exit_status: Option<ExitStatus>,
    /// Per-process signal control block.
    ///
    /// Holds the process-wide signal dispositions installed via `sigaction()`.
    signals: SignalControl,
    /// Job-control stopped flag. Set when a stop signal (`SIGSTOP`/`SIGTSTP`/`SIGTTIN`/`SIGTTOU`)
    /// suspends the process and cleared when `SIGCONT` resumes it. A stopped process is skipped by
    /// the scheduler — none of its threads run — until it is continued, modelling the POSIX
    /// *stopped* state without a dedicated scheduling list.
    stopped: bool,
}

impl ProcessState {
    pub(super) fn new(
        pid: ProcessIdentifier,
        parent: ProcessIdentifier,
        termination_credit: Option<LifecycleTerminationCredit>,
        vmem: Vmem,
    ) -> Self {
        Self {
            pid,
            parent,
            termination_credit,
            capabilities: Capabilities::default(),
            vmem,
            events: LinkedList::new(),
            mailbox: Mailbox::default(),
            delivery_cursor: 0,
            mmio: LinkedList::new(),
            pmio: LinkedList::new(),
            mutexes: BTreeMap::new(),
            conditions: BTreeMap::new(),
            pending_exit_status: None,
            signals: SignalControl::default(),
            stopped: false,
        }
    }

    pub fn pid(&self) -> ProcessIdentifier {
        self.pid
    }

    pub fn ppid(&self) -> ProcessIdentifier {
        self.parent
    }

    /// Takes the capacity credit reserved for this process's termination record.
    pub(super) fn take_termination_credit(&mut self) -> Option<LifecycleTerminationCredit> {
        self.termination_credit.take()
    }

    ///
    /// # Description
    ///
    /// Installs the capacity credit reserved for this process's termination record.
    ///
    /// # Parameters
    ///
    /// - `credit`: The capacity credit to install for the process's future termination record.
    ///
    /// # Panics
    ///
    /// This function panics if a termination credit is already installed.
    ///
    fn install_termination_credit(&mut self, credit: LifecycleTerminationCredit) {
        assert!(
            self.termination_credit.is_none(),
            "process termination credit was already installed"
        );
        self.termination_credit = Some(credit);
    }

    ///
    /// # Description
    ///
    /// Sets the pending exit status for the process if one is not already set. This is used
    /// when a thread calls `exit()` to terminate the process, but there are other threads that
    /// need to be terminated first. The exit status from the first thread that called `exit()`
    /// is preserved and used as the final process exit status.
    ///
    /// If a pending exit status is already set (from an earlier `exit()` call), this function
    /// does nothing, ensuring that the original exit status is not overwritten by subsequent
    /// threads being terminated.
    ///
    /// # Parameters
    ///
    /// - `status`: The exit status to store (only if not already set).
    ///
    pub fn set_pending_exit_status(&mut self, status: ExitStatus) {
        if self.pending_exit_status.is_none() {
            self.pending_exit_status = Some(status);
        }
    }

    ///
    /// # Description
    ///
    /// Takes the pending exit status, if any. Returns the stored exit status and clears it.
    /// This is called when the last thread in the process terminates to retrieve the exit
    /// status that was set by the thread that originally called `exit()`.
    ///
    /// # Returns
    ///
    /// The pending exit status if one was set, otherwise `None`.
    ///
    pub fn take_pending_exit_status(&mut self) -> Option<ExitStatus> {
        self.pending_exit_status.take()
    }

    pub fn set_capability(&mut self, capability: Capability) {
        self.capabilities.set(capability)
    }

    pub fn clear_capability(&mut self, capability: Capability) {
        self.capabilities.clear(capability)
    }

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities.has(capability)
    }

    pub fn vmem(&self) -> &Vmem {
        &self.vmem
    }

    pub fn vmem_mut(&mut self) -> &mut Vmem {
        &mut self.vmem
    }

    ///
    /// # Description
    ///
    /// Returns a mutable reference to the per-process signal control block.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`SignalControl`] of this process.
    ///
    pub fn signals_mut(&mut self) -> &mut SignalControl {
        &mut self.signals
    }

    ///
    /// # Description
    ///
    /// Returns an immutable reference to the per-process signal control block.
    ///
    /// # Returns
    ///
    /// An immutable reference to the [`SignalControl`] of this process.
    ///
    pub fn signals(&self) -> &SignalControl {
        &self.signals
    }

    ///
    /// # Description
    ///
    /// Replaces the per-process signal control block.
    ///
    /// Used by `fork()` to install the dispositions and restorer inherited from the parent into the
    /// freshly created child.
    ///
    /// # Parameters
    ///
    /// - `signals`: The signal control block to install.
    ///
    pub fn set_signals(&mut self, signals: SignalControl) {
        self.signals = signals;
    }

    ///
    /// # Description
    ///
    /// Reports whether the process is job-control *stopped*.
    ///
    /// # Returns
    ///
    /// `true` if the process is stopped (and therefore not schedulable until continued), `false`
    /// otherwise.
    ///
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    ///
    /// # Description
    ///
    /// Sets or clears the job-control *stopped* flag of the process.
    ///
    /// While set, the scheduler skips the process so none of its threads run; clearing it (on
    /// `SIGCONT`) makes the process schedulable again, resuming its threads from where they were
    /// suspended.
    ///
    /// # Parameters
    ///
    /// - `stopped`: `true` to stop the process, `false` to continue it.
    ///
    pub fn set_stopped(&mut self, stopped: bool) {
        self.stopped = stopped;
    }

    ///
    /// # Description
    ///
    /// Replaces the process's address space with `vmem`, returning the previous one.
    ///
    /// This is used by `execv()` to install a freshly built image's address space while keeping
    /// the rest of the process state (identity, capabilities) intact. The caller is responsible
    /// for reclaiming the returned address space once it is no longer the active one (i.e. after
    /// the context switch into the new image has loaded the new page directory).
    ///
    /// # Parameters
    ///
    /// - `vmem`: The new address space to install.
    ///
    /// # Returns
    ///
    /// The address space that was previously installed.
    ///
    pub fn replace_vmem(&mut self, vmem: Vmem) -> Vmem {
        core::mem::replace(&mut self.vmem, vmem)
    }

    ///
    /// # Description
    ///
    /// Checks whether the process owns any "special" resources that prevent it from being
    /// safely duplicated. A process is considered to own special resources when it holds any
    /// of the following:
    ///
    /// - One or more allocated memory-mapped I/O regions.
    /// - One or more allocated port-mapped I/O ports.
    /// - One or more event ownerships.
    /// - One or more in-flight (buffered) inter-process messages in its mailbox.
    ///
    /// Mutexes and condition variables are intentionally excluded because their addresses
    /// alias user-space objects and are recreated lazily on access from the cloned address
    /// space. Resources covered above, by contrast, are uniquely owned by the parent and
    /// cannot be safely transferred to a child via address-space cloning alone.
    ///
    /// # Scope
    ///
    /// This predicate only inspects per-process state. It does **not** inspect global state
    /// such as the rendezvous lists used by `push`/`pull`; threads belonging to this process
    /// that are currently sleeping on a rendezvous are not tracked here. Such pending
    /// rendezvous reference user buffers in the parent's address space only, so they remain
    /// correct after duplication: copy-on-write resolution on the kernel-side write paths
    /// (`vmcopy_user_to_user`, `copy_to_user_unaligned`) ensures wake-up writes hit the
    /// parent's private frames, not the child's.
    ///
    /// # Returns
    ///
    /// `true` if the process owns any of the resources listed above, otherwise `false`.
    ///
    pub fn has_special_resources(&self) -> bool {
        !self.mmio.is_empty()
            || !self.pmio.is_empty()
            || !self.events.is_empty()
            || !self.mailbox.is_empty()
    }

    pub fn copy_from_user_unaligned(
        &self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.vmem.copy_from_user_unaligned(dst, src, size)
    }

    pub fn copy_to_user_unaligned(
        &mut self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.vmem.copy_to_user_unaligned(dst, src, size)
    }

    pub fn add_event(&mut self, ownership: EventOwnership) {
        self.events.push_back(ownership)
    }

    ///
    /// # Description
    ///
    /// Removes the ownership guards that match an event being unregistered.
    ///
    /// # Parameters
    ///
    /// - `ev`: Event being unregistered.
    ///
    pub fn remove_event(&mut self, ev: &Event) {
        self.events
            .retain(|ownership| !event_guard_matches(ownership.event(), ev))
    }

    ///
    /// # Description
    ///
    /// Posts a message into this process's mailbox.
    ///
    /// # Parameters
    ///
    /// - `sequence`: Sequence number assigned to the message.
    /// - `message`: Message to be posted.
    ///
    pub fn post_message(&mut self, sequence: DeliverySequence, message: Message) {
        self.mailbox.send(sequence, message)
    }

    ///
    /// # Description
    ///
    /// Peeks the oldest mailbox message eligible for a thread without consuming it. A message is
    /// eligible when it is addressed either to the thread itself or to its process.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the receiving thread.
    ///
    /// # Returns
    ///
    /// The selected message and its delivery sequence, or [`None`] if no mailbox message is
    /// eligible for the thread.
    ///
    pub fn peek_message(&self, tid: ThreadIdentifier) -> Option<(DeliverySequence, Message)> {
        self.mailbox.peek(tid)
    }

    ///
    /// # Description
    ///
    /// Commits delivery of a mailbox message previously selected by [`Self::peek_message`]. The
    /// selected message is removed from the mailbox only when its delivery sequence still matches
    /// the oldest message eligible for the receiving thread.
    ///
    /// # Parameters
    ///
    /// - `tid`: Identifier of the receiving thread.
    /// - `sequence`: Delivery sequence returned by [`Self::peek_message`].
    ///
    /// # Returns
    ///
    /// `true` if the selected message was removed, or `false` if no mailbox message is eligible for
    /// the thread.
    ///
    /// # Panics
    ///
    /// This function panics if an eligible message exists but `sequence` does not identify it. This
    /// indicates a stale, duplicate, or otherwise invalid delivery token.
    ///
    pub fn commit_message(&mut self, tid: ThreadIdentifier, sequence: DeliverySequence) -> bool {
        self.mailbox.commit(tid, sequence)
    }

    ///
    /// # Description
    ///
    /// Returns this process's event-service cursor.
    ///
    /// # Returns
    ///
    /// This process's event-service cursor.
    ///
    pub fn delivery_cursor(&self) -> usize {
        self.delivery_cursor
    }

    ///
    /// # Description
    ///
    /// Sets this process's event-service cursor.
    ///
    /// # Parameters
    ///
    /// - `cursor`: New event-service cursor for this process.
    ///
    pub fn set_delivery_cursor(&mut self, cursor: usize) {
        self.delivery_cursor = cursor;
    }

    /// Removes every buffered message addressed exactly to `tid`.
    pub fn purge_thread_messages(&mut self, tid: ThreadIdentifier) -> usize {
        self.mailbox.purge_thread(tid)
    }

    /// Removes every buffered message.
    pub fn purge_messages(&mut self) -> usize {
        self.mailbox.purge_all()
    }

    /// # Description
    ///
    /// Adds an MMIO region to the process state.
    ///
    /// # Parameters
    ///
    /// - `region`: The I/O memory region to add.
    ///
    /// # Note
    ///
    /// Tag uniqueness is enforced by [`IoMemoryAllocator`], which guarantees that no two regions
    /// with the same tag can be allocated simultaneously.
    ///
    pub fn add_mmio(&mut self, region: IoMemoryRegion) {
        self.mmio.push_back(region)
    }

    ///
    /// # Description
    ///
    /// Removes the MMIO region identified by the given tag from the process state.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag that uniquely identifies the region to remove.
    ///
    /// # Note
    ///
    /// Tag uniqueness is enforced by [`IoMemoryAllocator`], so at most one region will match.
    ///
    pub fn remove_mmio(&mut self, tag: MmioTag) {
        if let Some(index) = self.mmio.iter().position(|r| r.tag() == tag) {
            // Remove only the first region that matches the given tag.
            self.mmio.remove(index);
        }
    }

    ///
    /// # Description
    ///
    /// Retrieves a reference to the MMIO region identified by the given tag.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag that uniquely identifies the region to look up.
    ///
    /// # Returns
    ///
    /// A reference to the [`IoMemoryRegion`] if found, or `None` otherwise.
    ///
    pub fn mmio_info(&self, tag: MmioTag) -> Option<&IoMemoryRegion> {
        self.mmio.iter().find(|r| r.tag() == tag)
    }

    pub fn add_pmio(&mut self, port: AnyIoPort) {
        self.pmio.push_back(port)
    }

    pub fn remove_pmio(&mut self, port_number: u16) -> Result<AnyIoPort, Error> {
        let index: Option<usize> = self.pmio.iter().position(|p| p.number() == port_number);
        match index {
            Some(index) => Ok(self.pmio.remove(index)),
            None => {
                let reason: &'static str = "io port not found";
                error!("{:?}", reason);
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }

    fn get_pmio(&self, port_number: u16) -> Result<&AnyIoPort, Error> {
        let port: Option<&AnyIoPort> = self.pmio.iter().find(|p| p.number() == port_number);
        match port {
            Some(port) => Ok(port),
            None => {
                let reason: &'static str = "io port not found";
                error!("{:?}", reason);
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }

    pub fn read_pmio(&self, port_number: u16, port_width: IoPortWidth) -> Result<u32, Error> {
        let port: &AnyIoPort = self.get_pmio(port_number)?;
        port.read(port_width)
    }

    pub fn write_pmio(
        &mut self,
        port_number: u16,
        port_width: IoPortWidth,
        value: u32,
    ) -> Result<(), Error> {
        let port: &mut AnyIoPort = self.get_pmio_mut(port_number)?;
        port.write(port_width, value)
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
    /// associated with the given address, a new mutex is created and returned.  On failure, an
    /// error is returned instead.
    ///
    pub fn get_mutex(&mut self, mutex_addr: MutexAddress) -> Result<Mutex, Error> {
        // Check if maximum number of mutexes has been reached.
        // Only reject if the mutex is not already present, since accessing an
        // existing entry does not grow the map.
        if !self.mutexes.contains_key(&mutex_addr) && self.mutexes.len() >= MUTEX_OPEN_MAX {
            let reason: &'static str = "maximum number of mutexes reached";
            error!("{:?} (addr={:#x?})", reason, mutex_addr);
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        Ok(self
            .mutexes
            .entry(mutex_addr)
            .or_insert_with(Mutex::new)
            .clone())
    }

    ///
    /// # Description
    ///
    /// Releases a mutex associated with the given address.
    ///
    /// # Parameters
    ///
    /// - `mtuex_addr`: Address of the mutex.
    ///
    /// # Returns
    ///
    /// Upon success, empty result is returned. Upon failure, an error is returned instead.
    ///
    pub fn put_mutex(&mut self, mutex_addr: MutexAddress) -> Result<(), Error> {
        // Check if mutex exists.
        if !self.mutexes.contains_key(&mutex_addr) {
            let reason: &'static str = "mutex not found";
            error!("{:?} (addr={:#x?})", reason, mutex_addr);
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let _: BTreeMap<_, _> = self
            .mutexes
            .extract_if(.., |addr, mutex| mutex_addr == *addr && mutex.reference_count() <= 2)
            .collect();

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
    pub fn get_cond(&mut self, cond_addr: ConditionAddress) -> Result<Condvar, Error> {
        // Check if maximum number of condition variables has been reached.
        // Only reject if the condvar is not already present, since accessing an
        // existing entry does not grow the map.
        if !self.conditions.contains_key(&cond_addr) && self.conditions.len() >= COND_OPEN_MAX {
            let reason: &'static str = "maximum number of condition variables reached";
            error!("{:?} (addr={:#x?})", reason, cond_addr);
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }

        Ok(self
            .conditions
            .entry(cond_addr)
            .or_insert_with(Condvar::new)
            .clone())
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
    /// Upon success, empty result is returned. Upon failure, an error is returned instead.
    ///
    pub fn put_cond(&mut self, cond_addr: ConditionAddress) -> Result<(), Error> {
        // Check if condition variable exists.
        if !self.conditions.contains_key(&cond_addr) {
            let reason: &'static str = "condition variable not found";
            error!("{:?} (addr={:#x?})", reason, cond_addr);
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let _: BTreeMap<_, _> = self
            .conditions
            .extract_if(.., |addr, cond| cond_addr == *addr && cond.reference_count() <= 1)
            .collect();

        Ok(())
    }

    fn get_pmio_mut(&mut self, port_number: u16) -> Result<&mut AnyIoPort, Error> {
        let port: Option<&mut AnyIoPort> = self.pmio.iter_mut().find(|p| p.number() == port_number);
        match port {
            Some(port) => Ok(port),
            None => {
                let reason: &'static str = "io port not found";
                error!("{:?}", reason);
                Err(Error::new(ErrorCode::NoSuchEntry, reason))
            },
        }
    }
}

impl ::core::fmt::Debug for ProcessState {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        write!(f, "{{ pid: {:?} }}", self.pid)
    }
}
