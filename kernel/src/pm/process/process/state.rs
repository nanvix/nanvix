// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    pm::process::{
        capability::Capabilities,
        identity::ProcessIdentity,
    },
};
use ::alloc::collections::LinkedList;
use ::kcall::{
    Capability,
    Error,
    ErrorCode,
    Event,
    GroupIdentifier,
    Message,
    ProcessIdentifier,
    UserIdentifier,
};

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
    /// Process identity.
    identity: ProcessIdentity,
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
}

impl ProcessState {
    pub fn new(pid: ProcessIdentifier, identity: ProcessIdentity, vmem: Vmem) -> Self {
        Self {
            pid,
            identity,
            capabilities: Capabilities::default(),
            vmem,
            events: LinkedList::new(),
            mailbox: Mailbox::default(),
            mmio: LinkedList::new(),
            pmio: LinkedList::new(),
        }
    }

    pub fn pid(&self) -> ProcessIdentifier {
        self.pid
    }

    pub fn identity(&self) -> &ProcessIdentity {
        &self.identity
    }

    pub fn get_uid(&self) -> UserIdentifier {
        self.identity.get_uid()
    }

    pub fn set_uid(&mut self, uid: UserIdentifier) -> Result<(), Error> {
        self.identity.set_uid(uid)
    }

    pub fn get_euid(&self) -> UserIdentifier {
        self.identity.get_euid()
    }

    pub fn set_euid(&mut self, euid: UserIdentifier) -> Result<(), Error> {
        self.identity.set_euid(euid)
    }

    pub fn get_gid(&self) -> GroupIdentifier {
        self.identity.get_gid()
    }

    pub fn set_gid(&mut self, gid: GroupIdentifier) -> Result<(), Error> {
        self.identity.set_gid(gid)
    }

    pub fn get_egid(&self) -> GroupIdentifier {
        self.identity.get_egid()
    }

    pub fn set_egid(&mut self, egid: GroupIdentifier) -> Result<(), Error> {
        self.identity.set_egid(egid)
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

    pub fn receive_message(&mut self) -> Option<Message> {
        self.mailbox.receive()
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
                Err(Error::new(ErrorCode::NoSuchEntry, &reason))
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
                Err(Error::new(ErrorCode::NoSuchEntry, &reason))
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

    fn get_pmio_mut(&mut self, port_number: u16) -> Result<&mut AnyIoPort, Error> {
        let port: Option<&mut AnyIoPort> = self.pmio.iter_mut().find(|p| p.number() == port_number);
        match port {
            Some(port) => Ok(port),
            None => {
                let reason: &'static str = "io port not found";
                error!("get_pmio_mut(): {:?}", reason);
                Err(Error::new(ErrorCode::NoSuchEntry, &reason))
            },
        }
    }
}
