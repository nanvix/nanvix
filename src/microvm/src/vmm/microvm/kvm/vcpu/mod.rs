// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod exit;
mod irqchip;
mod timer;

//==================================================================================================
// Exports
//==================================================================================================

use ::kvm_bindings::kvm_fpu;
pub use exit::*;

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::kvm::partition::VirtualPartition;
use ::anyhow::Result;
use ::kvm_bindings::{
    kvm_regs,
    kvm_sregs,
};
use ::kvm_ioctls::{
    VcpuExit,
    VcpuFd,
};
use ::std::sync::{
    Arc,
    Mutex,
};
use irqchip::IrqChip;
use timer::Timer;

//==================================================================================================
// Constants
//==================================================================================================

// Mask all fp-exception, set rounding to nearest, set precision to 64-bit
const FP_CONTROL_WORD_DEFAULT: u16 = 0x37f;
// Each 8 of x87 fpu registers is empty
const FP_TAG_WORD_DEFAULT: u8 = 0xff;
// Mask simd fp-exceptions, clear exception flags, set rounding to nearest, disable flush-to-zero mode, disable denormals-are-zero mode
const MXCSR_DEFAULT: u32 = 0x1f80;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a virtual processor.
///
pub struct VirtualProcessor {
    /// Handle to underlying virtual partition.
    _partition: Arc<Mutex<VirtualPartition>>,
    /// Handle to underlying virtual processor.
    fd: VcpuFd,
    /// Handle to underlying interrupt controller.
    _irqchip: IrqChip,
    /// Handle to timer.
    _timer: Timer,
    /// Processor state.
    online: bool,
    /// Exit status code.
    exit_status: u16,
}

impl VirtualProcessor {
    pub fn new(partition: Arc<Mutex<VirtualPartition>>, id: u64) -> Result<Self> {
        trace!("new(): id={}", id);
        crate::timer!("vcpu_creation");

        // Setup interrupt controller.
        let irqchip: IrqChip = IrqChip::new(&partition)?;
        // Create programmable interrupt timer.
        let timer: Timer = Timer::new(&partition)?;

        let fd: VcpuFd = partition
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?
            .vm()
            .create_vcpu(id)?;

        // Reset FPU state.
        let fpu = kvm_fpu {
            fcw: FP_CONTROL_WORD_DEFAULT,
            ftwx: FP_TAG_WORD_DEFAULT,
            mxcsr: MXCSR_DEFAULT,
            ..Default::default() // zero out the rest
        };
        fd.set_fpu(&fpu)?;

        Ok(Self {
            _partition: partition,
            fd,
            _irqchip: irqchip,
            _timer: timer,
            online: false,
            exit_status: 0,
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
    /// # Parameters
    ///
    /// - `exit_status`: Exit status code.
    ///
    pub fn poweroff(&mut self, exit_status: u16) {
        trace!("poweroff(): exit_status={}", exit_status);
        self.online = false;
        self.exit_status = exit_status;
    }

    ///
    /// # Description
    ///
    /// Gets the exit status code of the virtual processor.
    ///
    /// # Returns
    ///
    /// The exit status code of the virtual processor.
    ///
    pub fn exit_status(&self) -> u16 {
        self.exit_status
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
