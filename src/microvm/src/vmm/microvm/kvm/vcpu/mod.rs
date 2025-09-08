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

use ::arch::cpu::{
    cpuid::{
        CPUID_FEATURES,
        EdxFeature,
    },
    mxcrs::{
        DenormalOperationMask,
        DivideByZeroMask,
        OverflowMask,
        PrecisionMask,
        UnderflowMask,
    },
};
use ::kvm_bindings::{
    CpuId,
    KVM_MAX_CPUID_ENTRIES,
    kvm_fpu,
};
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
        trace!("new(): id={id}");
        crate::timer!("vcpu_creation");

        // Setup interrupt controller.
        let irqchip: IrqChip = IrqChip::new(&partition)?;
        // Create programmable interrupt timer.
        let timer: Timer = Timer::new(&partition)?;

        let mut fd: VcpuFd = partition
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .vm()
            .create_vcpu(id)?;

        Self::setup_pentium4_cpu_features(partition.clone(), &mut fd)?;

        // Reset FPU state.
        let fpu: kvm_fpu = kvm_fpu {
            fcw: FP_CONTROL_WORD_DEFAULT,
            ftwx: FP_TAG_WORD_DEFAULT,
            // Mask all SIMD exceptions.
            mxcsr: (PrecisionMask::Masked as u32)
                | (UnderflowMask::Masked as u32)
                | (OverflowMask::Masked as u32)
                | (DivideByZeroMask::Masked as u32)
                | (DenormalOperationMask::Masked as u32),
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
        trace!("reset(): rip={rip:#010x}, rax={rax:#010x}, rbx={rbx:#010x}");
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
        trace!("poweroff(): exit_status={exit_status}");
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
                warn!("run(): mmio read (addr={addr:#010x}, data.len={})", data.len());
                Ok(VirtualProcessorExitContext::Unknown)
            },
            // Write to a MMIO region.
            VcpuExit::MmioWrite(addr, data) => {
                // TODO: handle MMIO write.
                warn!("run(): mmio write (addr={addr:#010x}, data.len={})", data.len());
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
                warn!("run(): fail entry (reason={reason:?}, cpud={cpud})");
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
            // Virtual processor was interrupted.
            VcpuExit::Intr => {
                warn!("run(): interrupted");
                Ok(VirtualProcessorExitContext::Interrupted)
            },
            // Unsupported exit reason.
            VcpuExit::Unsupported(reason) => {
                // TODO: handle unsupported exit reason.
                warn!("run(): unsupported exit reason ({reason:?})");
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

    ///
    /// # Description
    ///
    /// Configures the virtual processor's CPU features to emulate a Pentium 4 processor.
    ///
    /// # Parameters
    ///
    /// - `partition`: Handle to the virtual partition.
    /// - `fd`: Handle to the virtual processor.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function returns empty. Otherwise, it returns an error.
    ///
    fn setup_pentium4_cpu_features(
        partition: Arc<Mutex<VirtualPartition>>,
        fd: &mut VcpuFd,
    ) -> Result<()> {
        let mut kvm_cpuid: CpuId = partition
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .kvm()
            .get_supported_cpuid(KVM_MAX_CPUID_ENTRIES)?;

        for entry in kvm_cpuid.as_mut_slice().iter_mut() {
            match entry.function {
                CPUID_FEATURES => {
                    entry.edx |= (EdxFeature::Fpu as u32) // FPU on-chip
                        | (EdxFeature::Vme as u32)        // Virtual-8086 Mode Enhancements
                        | (EdxFeature::De as u32)         // Debugging Extensions
                        | (EdxFeature::Pse as u32)        // Page Size Extension
                        | (EdxFeature::Tsc as u32)        // Time Stamp Counter
                        | (EdxFeature::Msr as u32)        // Model Specific Registers
                        | (EdxFeature::Pae as u32)        // Physical Address Extension
                        | (EdxFeature::Mce as u32)        // Machine Check Exception
                        | (EdxFeature::Cx8 as u32)        // CMPXCHG8B instruction
                        | (EdxFeature::Apic as u32)       // APIC on-chip
                        | (EdxFeature::Sep as u32)        // SYSENTER and SYSEXIT instructions
                        | (EdxFeature::Mtrr as u32)       // Memory Type Range Registers
                        | (EdxFeature::Pge as u32)        // Page Global Enable
                        | (EdxFeature::Mca as u32)        // Machine Check Architecture
                        | (EdxFeature::Cmov as u32)       // Conditional Move instructions
                        | (EdxFeature::Pat as u32)        // Page Attribute Table
                        | (EdxFeature::Pse36 as u32)      // 36-bit Page Size Extension
                        | (EdxFeature::Clflush as u32)    // CLFLUSH instruction
                        | (EdxFeature::Ds as u32)         // Debug Store
                        | (EdxFeature::Acpi as u32)       // Thermal Monitor and Software Controlled Clock
                        | (EdxFeature::Mmx as u32)        // MMX Instructions
                        | (EdxFeature::Fxsr as u32)       // FXSAVE/FXRSTOR instructions
                        | (EdxFeature::Sse as u32)        // SSE instructions
                        | (EdxFeature::Sse2 as u32)       // SSE2 instructions
                        | (EdxFeature::Ss as u32)         // Self Snoop
                        | (EdxFeature::Tm as u32)         // Thermal Monitor
                        | (EdxFeature::Pbe as u32)        // Pending Break Enable
                        ;
                },
                _ => continue,
            }
        }

        // Set CPUID and check for errors.
        if let Err(error) = fd.set_cpuid2(&kvm_cpuid) {
            let reason: String = format!("failed to set cpuid (error={error:?})");
            error!("setup_pentium4_cpu_features(): {reason}");
            return Err(anyhow::anyhow!(reason));
        }

        Ok(())
    }
}
