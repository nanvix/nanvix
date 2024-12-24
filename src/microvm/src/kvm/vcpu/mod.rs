// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod exit;

//==================================================================================================
// Exports
//==================================================================================================

pub use exit::*;

//==================================================================================================
// Imports
//==================================================================================================

use crate::kvm::partition::VirtualPartition;
use ::anyhow::Result;
use ::kvm_bindings::{
    kvm_regs,
    kvm_sregs,
};
use ::kvm_ioctls::{
    VcpuExit,
    VcpuFd,
};
use ::std::{
    cell::RefCell,
    rc::Rc,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a virtual processor.
///
pub struct VirtualProcessor {
    // Handle to underlying virtual partition.
    _partition: Rc<RefCell<VirtualPartition>>,
    // Handle to underlying virtual processor.
    fd: VcpuFd,
    // Processor state.
    online: bool,
}

impl VirtualProcessor {
    pub fn new(partition: Rc<RefCell<VirtualPartition>>, id: u64) -> Result<Self> {
        trace!("new(): id={}", id);
        crate::timer!("vcpu_creation");
        let fd: VcpuFd = partition.borrow().vm().create_vcpu(id)?;
        Ok(Self {
            _partition: partition,
            fd,
            online: false,
        })
    }

    ///
    /// # Description
    ///
    /// Resets the virtual processor.
    ///
    /// # Parameters
    ///
    /// - `rip`: Value to the the `rip` register.
    /// - `rax`: Value to set the `rax` register.
    /// - `rbx`: Value to set the `rbx` register.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn reset(&mut self, rip: u64, rax: u64, rbx: u64) -> Result<()> {
        trace!("reset(): rip={:#010x}, rax={:#010x}, rbx={:#010x}", rip, rax, rbx);
        crate::timer!("vcpu_reset");

        // Reset system registers.
        let mut vcpu_sregs: kvm_sregs = self.fd.get_sregs()?;
        vcpu_sregs.cs.base = 0;
        vcpu_sregs.cs.selector = 0;
        self.fd.set_sregs(&vcpu_sregs)?;

        // Reset general purpose registers.
        let mut vcpu_regs: kvm_regs = self.fd.get_regs()?;
        vcpu_regs.rip = rip;
        vcpu_regs.rax = rax;
        vcpu_regs.rbx = rbx;
        vcpu_regs.rflags = 2;
        self.fd.set_regs(&vcpu_regs)?;

        // Processor is now online.
        self.online = true;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Powers off the virtual processor.
    ///
    pub fn poweroff(&mut self) {
        trace!("poweroff()");
        self.online = false;
    }

    ///
    /// # Description
    ///
    /// Checks if the virtual processor is online.
    ///
    /// # Returns
    ///
    /// If the virtual processor is online, this method returns `true`. Otherwise, it returns
    /// `false` instead.
    pub fn is_online(&self) -> bool {
        self.online
    }

    ///
    /// # Description
    ///
    /// Runs the virtual processor until it exits.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the context in which the virtual processor
    /// exited. Otherwise, it returns an error.
    ///
    ///
    pub fn run(&mut self) -> Result<VirtualProcessorExitContext> {
        crate::timer!("vcpu_run");
        // Run the virtual processor and parse exit reason.
        match self.fd.run()? {
            // Read from an I/O port.
            VcpuExit::IoIn(port, data) => Ok(VirtualProcessorExitContext::PmioIn(port, data)),
            // Write to an I/O port.
            VcpuExit::IoOut(port, data) => {
                let mut value: u32 = 0;
                for (i, b) in data.iter().enumerate() {
                    value |= (*b as u32) << (i * 8);
                }
                Ok(VirtualProcessorExitContext::PmioOut(port, value, data.len()))
            },
            // Read from an MMIO region.
            VcpuExit::MmioRead(addr, data) => {
                // TODO: handle MMIO read.
                warn!("run(): mmio read (addr={:#010x}, data.len={})", addr, data.len());
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Write to a MMIO region.
            VcpuExit::MmioWrite(addr, data) => {
                // TODO: handle MMIO write.
                warn!("run(): mmio write (addr={:#010x}, data.len={})", addr, data.len());
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Exception occurred.
            VcpuExit::Exception => {
                // TODO: handle exception.
                warn!("run(): exception");
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Hypervisor call invoked.
            VcpuExit::Hypercall(_) => {
                // TODO: handle hypercall.
                warn!("run(): hypercall");
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Debugging event occurred.
            VcpuExit::Debug(_) => {
                // TODO: handle debug.
                warn!("run(): debug");
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Halt the virtual processor.
            VcpuExit::Hlt => Ok(VirtualProcessorExitContext::Halt),
            // Shutdown the virtual processor.
            VcpuExit::Shutdown => {
                // TODO: handle shutdown.
                warn!("run(): shutdown");
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Fail to run the virtual processor.
            VcpuExit::FailEntry(reason, cpud) => {
                // TODO: handle fail entry.
                warn!("run(): fail entry (reason={:?}, cpud={})", reason, cpud);
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Non-maskable interrupt occurred.
            VcpuExit::Nmi => {
                // TODO: handle NMI.
                warn!("run(): nmi");
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Internal error occurred.
            VcpuExit::InternalError => {
                // TODO: handle internal error.
                warn!("run(): internal error");
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Unsupported exit reason.
            VcpuExit::Unsupported(reason) => {
                // TODO: handle unsupported exit reason.
                warn!("run(): unsupported exit reason ({:?})", reason);
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Unknown exit reason.
            // NOTE: we do not parse all exit reasons, so it is worthy checking what happened.
            _ => {
                warn!("run(): unknown exit reason");
                Ok(VirtualProcessorExitContext::Unknown)
            },
        }
    }
}
