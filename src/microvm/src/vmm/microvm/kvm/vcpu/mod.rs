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

pub use exit::*;

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::kvm::partition::VirtualPartition;
use ::anyhow::Result;
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
    Msrs,
    Xsave,
    kvm_clock_data,
    kvm_debugregs,
    kvm_fpu,
    kvm_irqchip,
    kvm_lapic_state,
    kvm_mp_state,
    kvm_msr_entry,
    kvm_pit_state2,
    kvm_regs,
    kvm_sregs,
    kvm_vcpu_events,
    kvm_xcrs,
    kvm_xsave,
};
use ::kvm_ioctls::{
    Cap,
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
    sync::{
        Arc,
        Mutex,
        MutexGuard,
    },
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
    partition: Arc<Mutex<VirtualPartition>>,
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
    // `VirtualProcessor` direct state:
    /// Whether the processor is online.
    online: bool,
    /// Exit status code.
    exit_status: u16,

    // `VirtualProcessor` indirect state:
    // VcpuFd state:
    /// General purpose registers.
    regs: kvm_regs,
    /// System registers (segment registers, control registers, etc.).
    sregs: kvm_sregs,
    /// FPU/SIMD state. Natively a `kvm_bindings::kvm_fpu`.
    fpu: Vec<u8>,
    /// CPUID table. Natively a `kvm_bindings::CpuId`.
    cpuid: Vec<u8>,
    /// Local Advanced Programmable Interrupt Controller.
    lapic: kvm_lapic_state,
    /// Model-Specific RegisterS. Natively a `kvm_bindings::Msrs`
    msrs: Vec<u8>,
    /// MultiProcessing State.
    mp_state: kvm_mp_state,
    /// KVM's xsave struct (x86 only). Natively a `kvm_bindings::Xsave`.
    xsave: Vec<u8>,
    /// XCRS (x86 only).
    xcrs: kvm_xcrs,
    /// Debug registers (x86 only).
    debugregs: kvm_debugregs,
    /// Pending exceptions, interrupts, NMIs, and related states.
    vcpu_events: kvm_vcpu_events,
    /// TSC frequency in kHz.
    tsc_khz: u32,

    // VmFd state:
    /// Interrupt controller.
    irqchip: kvm_irqchip,
    /// Timer.
    pit_state: kvm_pit_state2,
    /// Timestamp of kvmclock.
    clock_data: kvm_clock_data,
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
            partition,
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
        match self.fd.run() {
            Ok(vcpu_exit) => match vcpu_exit {
                // Read from an I/O port.
                VcpuExit::IoIn(port, data) => {
                    Ok(VirtualProcessorExitContext::PmioIn(port, data.to_vec()))
                },
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
            },
            // vCPU thread was interrupted by a signal from the host.
            Err(e) if e.errno() == libc::EINTR => {
                warn!("run(): interrupted");
                Ok(VirtualProcessorExitContext::Interrupted)
            },
            Err(e) => Err(e.into()),
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

    ///
    /// # Description
    ///
    /// Captures the current state of the virtual processor.
    ///
    /// Both `IrqChip` and `Timer` are components of the `VirtualPartition`. They must be saved
    /// through the `VmFd` API (`get_irq_chip()` and `get_pit2()`). This function extracts state
    /// from `VcpuFd` and `VmFd`.
    ///
    /// # Returns
    ///
    /// Upon successful completion, returns the current processor state that can be serialized and
    /// saved to a file. Otherwise, returns an error.
    ///
    pub fn get_state(&self) -> Result<VirtualProcessorState> {
        // Plain getters:
        let regs: kvm_regs = match self.fd.get_regs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting kvm_regs (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let sregs: kvm_sregs = match self.fd.get_sregs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting sregs (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let fpu: kvm_fpu = match self.fd.get_fpu() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting fpu (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let cpuid = match self.fd.get_cpuid2(KVM_MAX_CPUID_ENTRIES) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting cpuid (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let lapic: kvm_lapic_state = match self.fd.get_lapic() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting lapic (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let mp_state: kvm_mp_state = match self.fd.get_mp_state() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting mp_state (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let xcrs: kvm_xcrs = match self.fd.get_xcrs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting xcrs (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let debugregs: kvm_debugregs = match self.fd.get_debug_regs() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting debugregs (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let vcpu_events: kvm_vcpu_events = match self.fd.get_vcpu_events() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting vcpu_events (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let tsc_khz: u32 = match self.fd.get_tsc_khz() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting tsc_khz (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // For the rest of the state, we need the partition's `Kvm` or `VmFd`.
        let locked_partition: MutexGuard<'_, VirtualPartition> = self
            .partition
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?;

        // Variable length (FAM):
        // Get KVM to find out the number of entries in FAMs:
        let kvm: &Kvm = locked_partition.kvm();

        // Build `Msrs` out of entries.
        let msr_index_list = match kvm.get_msr_feature_index_list() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting msr_index_list (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let msr_entries: Vec<kvm_msr_entry> = msr_index_list
            .as_slice()
            .iter()
            .map(|idx| kvm_msr_entry {
                index: *idx,
                data: 0,
                ..Default::default()
            })
            .collect();
        let mut msrs: Msrs = match Msrs::from_entries(&msr_entries) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed creating msrs (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        match self.fd.get_msrs(&mut msrs) {
            Ok(nmsrs_read) => {
                // Sanity check.
                if nmsrs_read != msr_entries.len() {
                    let reason: String = format!(
                        "`nmsrs_read`(={}) is different from `msr_entries.len()`(={})",
                        nmsrs_read,
                        msr_entries.len(),
                    );
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                }
            },
            Err(e) => {
                let reason: String = format!("failed mutating msrs (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // xsave can be either `Xsave` or `kvm_xsave`. Declaring it as `Vec<u8>` fits both.
        let xsave: Vec<u8> = if kvm.check_extension_int(Cap::Xsave2) > 0 {
            // Docs: https://docs.rs/kvm-bindings/0.14.0/kvm_bindings/struct.kvm_xsave2.html
            // KVM_CHECK_EXTENSION(KVM_CAP_XSAVE2) returns the total bytes for the whole structure.
            let xsave_total_bytes: usize = kvm.check_extension_int(Cap::Xsave2) as usize;
            // Fam-wrapper type Xsave is a wrapper over kvm_xsave2 (post-5.17) or kvm_xsave.
            let header_size: usize = mem::size_of::<kvm_bindings::kvm_xsave2>();
            let fam_entries: usize = xsave_total_bytes.saturating_sub(header_size);
            // Each Fam entry in kvm_xsave2 is u32 (per bindings).
            let fam_units: usize = fam_entries.div_ceil(mem::size_of::<u32>());
            let mut xsave2: Xsave = match Xsave::new(fam_units) {
                Ok(v) => v,
                Err(e) => {
                    let reason: String = format!("failed creating xsave2 (error={e:?})");
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            // SAFETY: This is safe because we've checked the number of elements before allocating.
            if let Err(e) = unsafe { self.fd.get_xsave2(&mut xsave2) } {
                let reason: String = format!("failed getting xsave2 (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            }
            match serialize_fam_struct(&xsave2) {
                Ok(xsave2) => xsave2,
                Err(e) => {
                    let reason: String = format!("failed serializing xsave2 (error={e:?})");
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                },
            }
        } else {
            // Older kernel that only supports fixed 4KB kvm_xsave.
            let small_xsave: kvm_xsave = match self.fd.get_xsave() {
                Ok(v) => v,
                Err(e) => {
                    let reason: String = format!("failed getting small_xsave (error={e:?})");
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            serialize_plain(&small_xsave)
        };

        // `VmFd` state:
        let vm: &VmFd = locked_partition.vm();
        let mut irqchip: kvm_irqchip = kvm_irqchip::default();
        if let Err(e) = vm.get_irqchip(&mut irqchip) {
            let reason: String = format!("failed getting irqchip (error={e:?})");
            error!("get_state(): {reason}");
            anyhow::bail!(reason)
        };
        let pit_state: kvm_pit_state2 = match vm.get_pit2() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting pit_state (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let clock_data: kvm_clock_data = match vm.get_clock() {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed getting clock_data (error={e:?})");
                error!("get_state(): {reason}");
                anyhow::bail!(reason)
            },
        };

        Ok(VirtualProcessorState {
            online: self.is_online(),
            exit_status: self.exit_status(),
            regs,
            sregs,
            fpu: serialize_plain(&fpu),
            cpuid: match serialize_fam_struct(&cpuid) {
                Ok(cpuid) => cpuid,
                Err(e) => {
                    let reason: String = format!("failed serializing cpuid (error={e:?})");
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                },
            },
            lapic,
            msrs: match serialize_fam_struct(&msrs) {
                Ok(msrs) => msrs,
                Err(e) => {
                    let reason: String = format!("failed serializing msrs (error={e:?})");
                    error!("get_state(): {reason}");
                    anyhow::bail!(reason)
                },
            },
            mp_state,
            xsave,
            xcrs,
            debugregs,
            vcpu_events,
            tsc_khz,
            irqchip,
            pit_state,
            clock_data,
        })
    }

    ///
    /// # Description
    ///
    /// Restores the virtual processor to a previously saved state.
    ///
    /// # Parameters
    ///
    /// - `state`: Processor state to restore.
    ///
    /// # Returns
    ///
    /// Upon successful completion, returns empty. Otherwise, returns an error.
    ///
    pub fn set_state(&mut self, _state: VirtualProcessorState) -> Result<()> {
        // TODO: set virtual processor state https://github.com/nanvix/nanvix/issues/948
        Ok(())
    }
}

impl VirtualProcessorState {
    pub fn validate(&self) -> Result<()> {
        // TODO: ensure state is safe to resume running.
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
    unsafe { slice::from_raw_parts((t as *const T) as *const u8, mem::size_of::<T>()).to_vec() }
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
fn serialize_fam_struct<T: Default + FamStruct>(
    wrapper: &FamStructWrapper<T>,
) -> Result<Vec<u8>, bincode::error::EncodeError> {
    let total_size: usize = mem::size_of_val(wrapper);
    // SAFETY: We're casting an object to a `&[u8]` of its own length, so the sizes match.
    let raw_bytes: &[u8] =
        unsafe { slice::from_raw_parts(wrapper as *const _ as *const u8, total_size) };
    bincode::serde::encode_to_vec(raw_bytes, bincode::config::standard())
}
