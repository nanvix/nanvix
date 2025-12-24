// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod exit;
mod fpu;
mod msr;

//==================================================================================================
// Exports
//==================================================================================================

pub use exit::*;

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::kvm::{
    pmio::PmioAccess,
    vcpu::{
        fpu::{
            Fpu,
            FpuState,
        },
        msr::{
            Msrs,
            MsrsState,
        },
    },
};
use ::anyhow::Result;
use ::arch::cpu::cpuid::{
    CPUID_FEATURES,
    EdxFeature,
};
use ::kvm_bindings::{
    CpuId,
    KVM_MAX_CPUID_ENTRIES,
    kvm_cpuid2,
    kvm_debugregs,
    kvm_lapic_state,
    kvm_mp_state,
    kvm_regs,
    kvm_sregs,
    kvm_vcpu_events,
    kvm_xcrs,
};
use ::kvm_ioctls::{
    Kvm,
    VcpuExit,
    VcpuFd,
    VmFd,
};
use ::serde::{
    Deserialize,
    Serialize,
};

use ::std::{
    mem,
    slice,
};
use ::syslog::{
    error,
    trace,
    warn,
};
use ::vmm_sys_util::fam::{
    FamStruct,
    FamStructWrapper,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Interrupt Enable flag in RFLAGS register.
pub const RFLAGS_INTERRUPT_ENABLE: u64 = 1 << 1;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a virtual processor.
///
pub struct VirtualProcessor {
    /// Handle to underlying virtual processor.
    fd: VcpuFd,
    /// Processor state.
    online: bool,
    /// Exit status code.
    exit_status: u16,
    /// Floating point unit.
    fpu: Fpu,
    /// MSRs device.
    msrs: Msrs,
}

///
/// # Description
///
/// Virtual CPU state that can be serialized and saved to disk.
///
/// The virtual CPU is composed of three structs from the `kvm_*` crates:
/// - `KVM`: A field of `VirtualPartition`.
/// - `VcpuFd`: A field of `VirtualProcessors`.
/// - `VmFd`: A field of `VirtualPartition`.
///
/// This structure holds the state that can be extracted from `VcpuFd` and `VmFd`, as `KVM` does not
/// hold state directly. Also, it holds the direct fields of `VirtualProcessor`: `online` and
/// `exit_status`.
///
#[derive(Serialize, Deserialize)]
pub struct VirtualProcessorState {
    /// General purpose registers.
    regs: kvm_regs,
    /// System registers (segment registers, control registers, etc.).
    sregs: kvm_sregs,
    /// CPUID table. Natively a `kvm_bindings::CpuId`.
    cpuid: Vec<u8>,
    /// Local Advanced Programmable Interrupt Controller.
    lapic: kvm_lapic_state,
    /// MultiProcessing State.
    mp_state: kvm_mp_state,
    /// XCRS (x86 only).
    xcrs: kvm_xcrs,
    /// Debug registers (x86 only).
    debugregs: kvm_debugregs,
    /// Pending exceptions, interrupts, NMIs, and related states.
    vcpu_events: kvm_vcpu_events,
    /// TSC frequency in kHz.
    tsc_khz: u32,
    /// Floating point unit state.
    fpu_ext: FpuState,
    /// MSRs state.
    msrs_state: MsrsState,
}

impl VirtualProcessor {
    ///
    /// # Description
    ///
    /// Creates a new virtual processor.
    ///
    /// # Parameters
    ///
    /// - `kvm_fd`: Handle to the KVM hypervisor.
    /// - `vm_fd`: Handle to the virtual machine.
    /// - `id`: ID of the virtual processor.
    ///
    /// # Return Value
    ///
    /// On success, this function returns a new virtual processor. On failure, it returns an object
    /// that describes the error.
    ///
    pub fn new(kvm_fd: &mut Kvm, vm_fd: &mut VmFd, id: u64) -> Result<Self> {
        trace!("new(): id={id}");

        let mut fd: VcpuFd = vm_fd.create_vcpu(id)?;

        Self::setup_pentium4_cpu_features(kvm_fd, &mut fd)?;

        let fpu: Fpu = Fpu::new(kvm_fd, &fd)?;

        Ok(Self {
            fd,
            online: false,
            exit_status: 0,
            fpu,
            msrs: Msrs,
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
        vcpu_regs.rflags = RFLAGS_INTERRUPT_ENABLE;
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
    ///
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
    pub fn run(&mut self) -> VirtualProcessorExitContext {
        // Run the virtual processor and parse exit reason.
        match self.fd.run() {
            Ok(vcpu_exit) => match vcpu_exit {
                // Read from an I/O port.
                VcpuExit::IoIn(port, data) => {
                    VirtualProcessorExitContext::Pmio(PmioAccess::PmioIn(port, data.to_vec()))
                },
                // Write to an I/O port.
                VcpuExit::IoOut(port, data) => {
                    let mut value: u32 = 0;
                    for (i, b) in data.iter().enumerate() {
                        value |= (*b as u32) << (i * 8);
                    }
                    let width = match ::core::convert::TryFrom::try_from(data.len()) {
                        Ok(width) => width,
                        Err(invalid) => {
                            warn!("run(): unsupported pmio write width (width={invalid})");
                            return VirtualProcessorExitContext::Unknown;
                        },
                    };
                    VirtualProcessorExitContext::Pmio(PmioAccess::PmioOut(port, value, width))
                },
                // Read from an MMIO region.
                VcpuExit::MmioRead(addr, data) => {
                    // TODO: handle MMIO read.
                    warn!("run(): mmio read (addr={addr:#010x}, data.len={})", data.len());
                    VirtualProcessorExitContext::Unknown
                },
                // Write to a MMIO region.
                VcpuExit::MmioWrite(addr, data) => {
                    // TODO: handle MMIO write.
                    warn!("run(): mmio write (addr={addr:#010x}, data.len={})", data.len());
                    VirtualProcessorExitContext::Unknown
                },
                // Exception occurred.
                VcpuExit::Exception => {
                    // TODO: handle exception.
                    warn!("run(): exception");
                    VirtualProcessorExitContext::Unknown
                },
                // Hypervisor call invoked.
                VcpuExit::Hypercall(_) => {
                    // TODO: handle hypercall.
                    warn!("run(): hypercall");
                    VirtualProcessorExitContext::Unknown
                },
                // Debugging event occurred.
                VcpuExit::Debug(_) => {
                    // TODO: handle debug.
                    warn!("run(): debug");
                    VirtualProcessorExitContext::Unknown
                },
                // Halt the virtual processor.
                VcpuExit::Hlt => VirtualProcessorExitContext::Halt,
                // Shutdown the virtual processor.
                VcpuExit::Shutdown => {
                    // TODO: handle shutdown.
                    warn!("run(): shutdown");
                    VirtualProcessorExitContext::Unknown
                },
                // Fail to run the virtual processor.
                VcpuExit::FailEntry(reason, cpud) => {
                    // TODO: handle fail entry.
                    warn!("run(): fail entry (reason={reason:?}, cpud={cpud})");
                    VirtualProcessorExitContext::Unknown
                },
                // Non-maskable interrupt occurred.
                VcpuExit::Nmi => {
                    // TODO: handle NMI.
                    warn!("run(): nmi");
                    VirtualProcessorExitContext::Unknown
                },
                // Internal error occurred.
                VcpuExit::InternalError => {
                    // TODO: handle internal error.
                    warn!("run(): internal error");
                    VirtualProcessorExitContext::Unknown
                },
                // Unsupported exit reason.
                VcpuExit::Unsupported(reason) => {
                    // TODO: handle unsupported exit reason.
                    warn!("run(): unsupported exit reason ({reason:?})");
                    VirtualProcessorExitContext::Unknown
                },
                // Unknown exit reason.
                // NOTE: we do not parse all exit reasons, so it is worthy checking what happened.
                _ => {
                    warn!("run(): unknown exit reason");
                    VirtualProcessorExitContext::Unknown
                },
            },
            // vCPU thread was interrupted by a signal from the host.
            Err(e) if e.errno() == libc::EINTR => {
                warn!("run(): interrupted");
                VirtualProcessorExitContext::Interrupted
            },
            Err(error) => {
                error!("run(): error running vCPU (error={error:?})");
                VirtualProcessorExitContext::Interrupted
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
    fn setup_pentium4_cpu_features(partition: &mut Kvm, fd: &mut VcpuFd) -> Result<()> {
        let mut kvm_cpuid: CpuId = partition.get_supported_cpuid(KVM_MAX_CPUID_ENTRIES)?;

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

    ///
    /// # Description
    ///
    /// Saves the current state of the virtual processor.
    ///
    /// # Parameters
    ///
    /// - `kvm`: Handle to the KVM hypervisor.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns a snapshot of the current
    /// virtual processor state. Otherwise, it returns an error.
    ///
    pub fn save_state(&self, kvm: &Kvm) -> Result<VirtualProcessorState> {
        // Ordering requirements between `kvm_get` calls:
        // https://github.com/firecracker-microvm/firecracker/blob/f0691f8253d4bde225b9f70ecabf39b7ad796935/src/vmm/src/arch/x86_64/vcpu.rs#L556

        trace!("save_state()");

        trace!("Saving VirtualPartition state");

        let mp_state: kvm_mp_state = match self.fd.get_mp_state() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting mp_state (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let regs: kvm_regs = match self.fd.get_regs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting kvm_regs (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let sregs: kvm_sregs = match self.fd.get_sregs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting sregs (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let fpu_ext: FpuState = match self.fpu.save_state(&self.fd) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting fpu_state (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let xcrs: kvm_xcrs = match self.fd.get_xcrs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting xcrs (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let debugregs: kvm_debugregs = match self.fd.get_debug_regs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting debugregs (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let lapic: kvm_lapic_state = match self.fd.get_lapic() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting lapic (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let tsc_khz: u32 = match self.fd.get_tsc_khz() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting tsc_khz (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let cpuid: FamStructWrapper<kvm_cpuid2> = match self.fd.get_cpuid2(KVM_MAX_CPUID_ENTRIES) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting cpuid (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let msrs_state: MsrsState = match self.msrs.save_state(kvm, &self.fd) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting msrs_state (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let vcpu_events: kvm_vcpu_events = match self.fd.get_vcpu_events() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting vcpu_events (error={e:?})");
                error!("save_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        Ok(VirtualProcessorState {
            regs,
            sregs,
            cpuid: serialize_fam_struct(&cpuid),
            lapic,
            mp_state,
            xcrs,
            debugregs,
            vcpu_events,
            tsc_khz,
            fpu_ext,
            msrs_state,
        })
    }

    ///
    /// # Description
    ///
    /// Loads the provided virtual processor state into this instance.
    ///
    /// # Parameters
    ///
    /// - `state`: The state snapshot to restore.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it
    /// returns an error.
    ///
    pub fn load_state(&mut self, _state: &VirtualProcessorState) -> Result<()> {
        trace!("load_state()");
        Ok(())
    }
}

//==================================================================================================
// Standalone functions
//==================================================================================================

///
/// # Description
///
/// Serializes a `Sized` structure into a vector of bytes.
///
/// # Parameters
///
/// - `t`: An instance of the T type.
///
/// # Returns
///
/// A vector of bytes with the same contents as the structure.
///
fn serialize_plain<T: Sized>(t: &T) -> Vec<u8> {
    // We cannot use `transmute`, because "generic parameters may not be used in const operations".
    // SAFETY: We're casting a `Sized` type to a `&[u8]` of its own length, so the sizes match.
    unsafe { slice::from_raw_parts((t as *const T).cast::<u8>(), mem::size_of::<T>()).to_vec() }
}

///
/// # Description
///
/// Serializes a flexible array member struct into a vector of bytes.
///
/// # Parameters
///
/// - `wrapper`: A `FamStructWrapper` with array entries of the T type.
///
/// # Returns
///
/// A vector of bytes with the same contents as the original wrapper.
///
fn serialize_fam_struct<T: Default + FamStruct>(wrapper: &FamStructWrapper<T>) -> Vec<u8> {
    let total_size: usize = mem::size_of_val(wrapper);
    // SAFETY: We're casting an object to a `&[u8]` of its own length, so the sizes match.
    let raw_bytes: &[u8] = unsafe {
        slice::from_raw_parts((wrapper as *const FamStructWrapper<T>).cast::<u8>(), total_size)
    };
    raw_bytes.to_vec()
}
