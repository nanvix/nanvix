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

mod interrupted;
mod runnable;
mod running;
mod sleeping;
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
        },
        mem::{
            PageAligned,
            VirtualAddress,
        },
    },
    ipc::Mailbox,
    mm::Vmem,
    pm::{
        process::capability::Capabilities,
        sync::{
            condvar::Condvar,
            mutex::Mutex,
        },
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
};

//==================================================================================================
// Exports
//==================================================================================================

pub use interrupted::InterruptedProcess;
pub use runnable::RunnableProcess;
pub use running::RunningProcess;
pub use sleeping::SleepingProcess;
pub use zombie::ZombieProcess;

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
    /// Capabilities.
    capabilities: Capabilities,
    /// Memory address space.
    vmem: Vmem,
    /// Event ownerships.
    events: LinkedList<EventOwnership>,
    /// Incoming messages.
    mailbox: Mailbox,
    /// Memory mapped I/O regions.
    mmio: LinkedList<IoMemoryRegion>,
    /// I/O ports.
    pmio: LinkedList<AnyIoPort>,
    /// Mutexes.
    mutexes: BTreeMap<MutexAddress, Mutex>,
    /// Condition variables.
    conditions: BTreeMap<ConditionAddress, Condvar>,
}

impl ProcessState {
    pub fn new(pid: ProcessIdentifier, vmem: Vmem) -> Self {
        Self {
            pid,
            capabilities: Capabilities::default(),
            vmem,
            events: LinkedList::new(),
            mailbox: Mailbox::default(),
            mmio: LinkedList::new(),
            pmio: LinkedList::new(),
            mutexes: BTreeMap::new(),
            conditions: BTreeMap::new(),
        }
    }

    pub fn pid(&self) -> ProcessIdentifier {
        self.pid
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

    pub fn copy_from_user_unaligned(
        &self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.vmem.copy_from_user_unaligned(dst, src, size)
    }

    pub fn copy_to_user_unaligned(
        &self,
        dst: VirtualAddress,
        src: VirtualAddress,
        size: usize,
    ) -> Result<(), Error> {
        self.vmem.copy_to_user_unaligned(dst, src, size)
    }

    pub fn add_event(&mut self, ownership: EventOwnership) {
        self.events.push_back(ownership)
    }

    pub fn remove_event(&mut self, ev: &Event) {
        self.events.retain(|o| o.event() != ev)
    }

    pub fn post_message(&mut self, message: Message) {
        self.mailbox.send(message)
    }

    pub fn receive_message(&mut self, tid: ThreadIdentifier) -> Option<Message> {
        self.mailbox.receive(tid)
    }

    pub fn add_mmio(&mut self, region: IoMemoryRegion) {
        self.mmio.push_back(region)
    }

    pub fn remove_mmio(&mut self, addr: PageAligned<VirtualAddress>) {
        self.mmio.retain(|r| r.base() != addr)
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
                error!("remove_pmio(): {:?}", reason);
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
                error!("get_pmio(): {:?}", reason);
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
        if self.mutexes.len() >= MUTEX_OPEN_MAX {
            let reason: &'static str = "maximum number of mutexes reached";
            error!("get_mutex(): {:?} (addr={:#x?})", reason, mutex_addr);
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
            error!("put_mutex(): {:?} (addr={:#x?})", reason, mutex_addr);
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let _: BTreeMap<_, _> = self
            .mutexes
            .extract_if(|&addr, mutex| mutex_addr == addr && mutex.reference_count() <= 2)
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
        if self.conditions.len() >= COND_OPEN_MAX {
            let reason: &'static str = "maximum number of condition variables reached";
            error!("get_condition(): {:?} (addr={:#x?})", reason, cond_addr);
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
            error!("put_condition(): {:?} (addr={:#x?})", reason, cond_addr);
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        let _: BTreeMap<_, _> = self
            .conditions
            .extract_if(|&addr, cond| cond_addr == addr && cond.reference_count() <= 1)
            .collect();

        Ok(())
    }

    fn get_pmio_mut(&mut self, port_number: u16) -> Result<&mut AnyIoPort, Error> {
        let port: Option<&mut AnyIoPort> = self.pmio.iter_mut().find(|p| p.number() == port_number);
        match port {
            Some(port) => Ok(port),
            None => {
                let reason: &'static str = "io port not found";
                error!("get_pmio_mut(): {:?}", reason);
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
