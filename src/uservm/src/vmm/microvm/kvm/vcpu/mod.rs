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
use ::arch::{
    cpu::cpuid::{
        CPUID_FEATURES,
        EdxFeature,
    },
    mem::PAGE_SIZE,
};
use ::kvm_bindings::{
    CpuId,
    KVM_MAX_CPUID_ENTRIES,
    kvm_cpuid_entry2,
    kvm_cpuid2,
    kvm_debugregs,
    kvm_lapic_state,
    kvm_mp_state,
    kvm_msr_entry,
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

use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    mem,
    slice,
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

/// MSR for KVM pvclock system time (new version).
/// Writing a GPA with bit 0 set enables KVM to populate a `KvmPvclockVcpuTimeInfo`
/// structure at that address.
const MSR_KVM_SYSTEM_TIME_NEW: u32 = 0x4b564d01;

/// IA32_TSC MSR — holds the current value of the Time Stamp Counter.
/// Writing this MSR sets the guest's TSC to the specified value.
const MSR_IA32_TSC: u32 = 0x10;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Hypervisor-independent segment register descriptor for diagnostic dumps.
///
pub struct SegmentRegister {
    /// Segment selector.
    pub selector: u16,
    /// Segment base address.
    pub base: u64,
    /// Segment limit.
    pub limit: u32,
}

///
/// # Description
///
/// Hypervisor-independent descriptor table register for diagnostic dumps.
///
pub struct DescriptorTable {
    /// Table base address.
    pub base: u64,
    /// Table limit.
    pub limit: u16,
}

///
/// # Description
///
/// Hypervisor-independent snapshot of virtual processor register state for diagnostic dumps.
///
pub struct VirtualProcessorDumpInfo {
    /// Instruction pointer.
    pub rip: u64,
    /// Flags register.
    pub rflags: u64,
    /// General-purpose registers.
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    /// Stack pointer.
    pub rsp: u64,
    /// Base (frame) pointer.
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    /// Control registers.
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub cr8: u64,
    /// Extended feature enable register.
    pub efer: u64,
    /// Segment registers.
    pub cs: SegmentRegister,
    pub ds: SegmentRegister,
    pub ss: SegmentRegister,
    pub es: SegmentRegister,
    pub fs: SegmentRegister,
    pub gs: SegmentRegister,
    /// Descriptor table registers.
    pub gdt: DescriptorTable,
    pub idt: DescriptorTable,
    /// Task register.
    pub tr: SegmentRegister,
    /// Local descriptor table register.
    pub ldt: SegmentRegister,
}

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
        Self::setup_rdtsc(&mut fd)?;

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

    /// Returns a pointer to KVM's immediate-exit byte for this vCPU.
    pub fn immediate_exit_ptr(&mut self) -> *mut u8 {
        &mut self.fd.get_kvm_run().immediate_exit
    }

    ///
    /// # Description
    ///
    /// Sets up the KVM paravirtualized clock for the guest.
    ///
    /// Enables the `MSR_KVM_SYSTEM_TIME_NEW` MSR so that KVM populates a shared
    /// memory page with TSC calibration data. The guest reads this page along
    /// with the CPU's TSC to compute the current time without VM exits.
    ///
    /// # Parameters
    ///
    /// - `clock_page_gpa`: Guest physical address of the pvclock page (must be page-aligned).
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn setup_pvclock(&mut self, clock_page_gpa: u64) -> Result<()> {
        trace!("setup_pvclock(): clock_page_gpa={clock_page_gpa:#010x}");

        // Ensure that the pvclock page GPA is page-aligned. Bit 0 is used as the enable flag
        // in the MSR value, and KVM expects the remaining bits to contain a page-aligned GPA.
        if !clock_page_gpa.is_multiple_of(PAGE_SIZE as u64) {
            let reason: String = format!(
                "pvclock page GPA is not page-aligned (clock_page_gpa={clock_page_gpa:#018x})"
            );
            error!("setup_pvclock(): {reason}");
            anyhow::bail!(reason);
        }

        // Bit 0 = enable.
        let msr_value: u64 = clock_page_gpa | 1;

        let msr_entries: Vec<kvm_msr_entry> = vec![kvm_msr_entry {
            index: MSR_KVM_SYSTEM_TIME_NEW,
            data: msr_value,
            ..Default::default()
        }];

        let msrs: ::kvm_bindings::Msrs = match ::kvm_bindings::Msrs::from_entries(&msr_entries) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed to create MSRs for pvclock (error={e:?})");
                error!("setup_pvclock(): {reason}");
                anyhow::bail!(reason)
            },
        };

        match self.fd.set_msrs(&msrs) {
            Ok(written) if written == msr_entries.len() => {
                trace!("setup_pvclock(): pvclock MSR set (value={msr_value:#018x})");
                Ok(())
            },
            Ok(written) => {
                let reason: String = format!(
                    "failed to set all pvclock MSRs: written {written} of {} entries",
                    msr_entries.len()
                );
                error!("setup_pvclock(): {reason}");
                anyhow::bail!(reason)
            },
            Err(e) => {
                let reason: String = format!("failed to set pvclock MSR (error={e:?})");
                error!("setup_pvclock(): {reason}");
                anyhow::bail!(reason)
            },
        }
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

    //==============================================================================================
    // GDB Debug Methods
    //==============================================================================================

    /// Enables or disables KVM guest debug mode.
    ///
    /// When enabled with `KVM_GUESTDBG_ENABLE | KVM_GUESTDBG_USE_SW_BP`, the guest will exit with
    /// `VcpuExit::Debug` when hitting an `INT3` instruction. Adding `KVM_GUESTDBG_SINGLESTEP`
    /// causes the guest to exit after every instruction.
    #[cfg(feature = "gdb")]
    pub fn set_guest_debug(&self, dbg: &kvm_bindings::kvm_guest_debug) -> Result<()> {
        self.fd
            .set_guest_debug(dbg)
            .map_err(|e| anyhow::anyhow!("failed to set guest debug (error={e:?})"))
    }

    /// Returns the current general-purpose registers of the virtual processor.
    pub fn get_regs(&self) -> Result<kvm_regs> {
        self.fd
            .get_regs()
            .map_err(|e| anyhow::anyhow!("failed to get regs (error={e:?})"))
    }

    /// Sets the general-purpose registers of the virtual processor.
    #[cfg(feature = "gdb")]
    pub fn set_regs(&self, regs: &kvm_regs) -> Result<()> {
        self.fd
            .set_regs(regs)
            .map_err(|e| anyhow::anyhow!("failed to set regs (error={e:?})"))
    }

    /// Returns the current segment and control registers of the virtual processor.
    pub fn get_sregs(&self) -> Result<kvm_sregs> {
        self.fd
            .get_sregs()
            .map_err(|e| anyhow::anyhow!("failed to get sregs (error={e:?})"))
    }

    ///
    /// # Description
    ///
    /// Returns a hypervisor-independent snapshot of the virtual processor's register state for
    /// diagnostic dumps.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns a [`VirtualProcessorDumpInfo`]. Otherwise,
    /// it returns an error.
    ///
    pub fn get_dump_info(&self) -> Result<VirtualProcessorDumpInfo> {
        let regs: kvm_regs = self.fd.get_regs().map_err(|e| {
            let reason: String = format!("failed to get registers (error={e:?})");
            error!("get_dump_info(): {reason}");
            anyhow::anyhow!(reason)
        })?;
        let sregs: kvm_sregs = self.fd.get_sregs().map_err(|e| {
            let reason: String = format!("failed to get special registers (error={e:?})");
            error!("get_dump_info(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        Ok(VirtualProcessorDumpInfo {
            rip: regs.rip,
            rflags: regs.rflags,
            rax: regs.rax,
            rbx: regs.rbx,
            rcx: regs.rcx,
            rdx: regs.rdx,
            rsi: regs.rsi,
            rdi: regs.rdi,
            rsp: regs.rsp,
            rbp: regs.rbp,
            r8: regs.r8,
            r9: regs.r9,
            r10: regs.r10,
            r11: regs.r11,
            r12: regs.r12,
            r13: regs.r13,
            r14: regs.r14,
            r15: regs.r15,
            cr0: sregs.cr0,
            cr2: sregs.cr2,
            cr3: sregs.cr3,
            cr4: sregs.cr4,
            cr8: sregs.cr8,
            efer: sregs.efer,
            cs: SegmentRegister {
                selector: sregs.cs.selector,
                base: sregs.cs.base,
                limit: sregs.cs.limit,
            },
            ds: SegmentRegister {
                selector: sregs.ds.selector,
                base: sregs.ds.base,
                limit: sregs.ds.limit,
            },
            ss: SegmentRegister {
                selector: sregs.ss.selector,
                base: sregs.ss.base,
                limit: sregs.ss.limit,
            },
            es: SegmentRegister {
                selector: sregs.es.selector,
                base: sregs.es.base,
                limit: sregs.es.limit,
            },
            fs: SegmentRegister {
                selector: sregs.fs.selector,
                base: sregs.fs.base,
                limit: sregs.fs.limit,
            },
            gs: SegmentRegister {
                selector: sregs.gs.selector,
                base: sregs.gs.base,
                limit: sregs.gs.limit,
            },
            gdt: DescriptorTable {
                base: sregs.gdt.base,
                limit: sregs.gdt.limit,
            },
            idt: DescriptorTable {
                base: sregs.idt.base,
                limit: sregs.idt.limit,
            },
            tr: SegmentRegister {
                selector: sregs.tr.selector,
                base: sregs.tr.base,
                limit: sregs.tr.limit,
            },
            ldt: SegmentRegister {
                selector: sregs.ldt.selector,
                base: sregs.ldt.base,
                limit: sregs.ldt.limit,
            },
        })
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
        let ctx: VirtualProcessorExitContext = match self.fd.run() {
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
                // Halt the virtual processor.
                VcpuExit::Hlt => VirtualProcessorExitContext::Halt,
                // Exception occurred.
                VcpuExit::Exception => {
                    warn!("run(): exception");
                    VirtualProcessorExitContext::Unknown
                },
                // Hypervisor call invoked.
                VcpuExit::Hypercall(_) => {
                    warn!("run(): hypercall");
                    VirtualProcessorExitContext::Unknown
                },
                // Debugging event occurred.
                VcpuExit::Debug(_) => VirtualProcessorExitContext::DebugEvent,
                // Shutdown the virtual processor (e.g., triple fault).
                VcpuExit::Shutdown => {
                    warn!("run(): shutdown");
                    VirtualProcessorExitContext::Shutdown
                },
                // Fail to run the virtual processor.
                VcpuExit::FailEntry(reason, cpud) => {
                    warn!("run(): fail entry (reason={reason:?}, cpud={cpud})");
                    VirtualProcessorExitContext::Unknown
                },
                // Non-maskable interrupt occurred.
                VcpuExit::Nmi => {
                    warn!("run(): nmi");
                    VirtualProcessorExitContext::Unknown
                },
                // Internal error occurred.
                VcpuExit::InternalError => {
                    warn!("run(): internal error");
                    VirtualProcessorExitContext::Unknown
                },
                // Unsupported exit reason.
                VcpuExit::Unsupported(reason) => {
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
            // vCPU thread was interrupted by a signal from the host.  This is the expected
            // mechanism for both orchestrator-driven shutdown (SIGUSR1) and profiler sampling
            // (SIGUSR2). Use trace! to avoid flooding logs when the profiler runs at high frequency
            // (e.g., 10kHz).
            Err(e) if e.errno() == libc::EINTR => {
                trace!("run(): interrupted");
                VirtualProcessorExitContext::Interrupted
            },
            Err(error) => {
                error!("run(): error running vCPU (error={error:?})");
                VirtualProcessorExitContext::Unknown
            },
        };

        ctx
    }

    ///
    /// # Description
    ///
    /// Initializes the guest's TSC by writing `IA32_TSC` MSR to zero.
    ///
    /// This ensures the guest has a deterministic RDTSC starting point and
    /// that the TSC feature is functional for pvclock calibration.
    ///
    /// # Parameters
    ///
    /// - `fd`: Handle to the virtual processor.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this function returns empty. Otherwise, it returns an error.
    ///
    fn setup_rdtsc(fd: &mut VcpuFd) -> Result<()> {
        let msr_entries: Vec<kvm_msr_entry> = vec![kvm_msr_entry {
            index: MSR_IA32_TSC,
            data: 0,
            ..Default::default()
        }];

        let msrs: ::kvm_bindings::Msrs = match ::kvm_bindings::Msrs::from_entries(&msr_entries) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed to create MSRs for RDTSC (error={e:?})");
                error!("setup_rdtsc(): {reason}");
                anyhow::bail!(reason)
            },
        };

        match fd.set_msrs(&msrs) {
            Ok(written) if written == msr_entries.len() => {
                trace!("setup_rdtsc(): IA32_TSC MSR set to 0");
                Ok(())
            },
            Ok(written) => {
                let reason: String = format!(
                    "failed to set all RDTSC MSRs: written {written} of {} entries",
                    msr_entries.len()
                );
                error!("setup_rdtsc(): {reason}");
                anyhow::bail!(reason)
            },
            Err(e) => {
                let reason: String = format!("failed to set IA32_TSC MSR (error={e:?})");
                error!("setup_rdtsc(): {reason}");
                anyhow::bail!(reason)
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
    pub fn load_state(&mut self, state: &VirtualProcessorState) -> Result<()> {
        // Ordering requirements between `kvm_set` calls (mirrors Firecracker — see
        // https://github.com/firecracker-microvm/firecracker/blob/f0691f8253d4bde225b9f70ecabf39b7ad796935/src/vmm/src/arch/x86_64/vcpu.rs#L654):
        //
        // - SET_CPUID and SET_MP_STATE depend on kvm_vcpu_is_bsp() and must therefore run before
        //   anything else.
        // - SET_REGS clears pending exceptions unconditionally, so it must come before
        //   SET_VCPU_EVENTS (which restores them).
        // - SET_LAPIC must come after SET_SREGS (which restores the APIC base MSR) and before
        //   SET_MSRS (the TSC deadline MSR only restores successfully when the LAPIC is
        //   already configured). Setting MSRs before the LAPIC silently leaves the TSC
        //   deadline at zero, firing the timer IRQ immediately on resume and
        //   deterministically panicking the kernel.
        trace!("load_state()");

        // 1. CPUID first (BSP dependency).
        let cpuid: CpuId = deserialize_cpuid(&state.cpuid)?;
        if let Err(e) = self.fd.set_cpuid2(&cpuid) {
            let reason: String = format!("failed setting cpuid (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 2. MP state (BSP dependency).
        if let Err(e) = self.fd.set_mp_state(state.mp_state) {
            let reason: String = format!("failed setting mp_state (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 3. General purpose registers (must precede SET_VCPU_EVENTS).
        if let Err(e) = self.fd.set_regs(&state.regs) {
            let reason: String = format!("failed setting regs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 4. System registers (must precede SET_LAPIC because it restores the APIC base MSR).
        if let Err(e) = self.fd.set_sregs(&state.sregs) {
            let reason: String = format!("failed setting sregs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 5. FPU/XSAVE state.
        if let Err(e) = self.fpu.load_state(&self.fd, &state.fpu_ext) {
            let reason: String = format!("failed setting fpu state (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 6. Extended control registers (XCR0).
        if let Err(e) = self.fd.set_xcrs(&state.xcrs) {
            let reason: String = format!("failed setting xcrs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 7. Debug registers.
        if let Err(e) = self.fd.set_debug_regs(&state.debugregs) {
            let reason: String = format!("failed setting debugregs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 8. LAPIC (after sregs, before msrs).
        if let Err(e) = self.fd.set_lapic(&state.lapic) {
            let reason: String = format!("failed setting lapic (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 9. TSC frequency (before msrs — IA32_TSC restore relies on the freq being set).
        if let Err(e) = self.fd.set_tsc_khz(state.tsc_khz) {
            let reason: String = format!("failed setting tsc_khz (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 10. MSRs (after lapic, after tsc_khz).
        if let Err(e) = self.msrs.load_state(&self.fd, &state.msrs_state) {
            let reason: String = format!("failed setting msrs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // 11. vCPU events last (after regs cleared exceptions).
        if let Err(e) = self.fd.set_vcpu_events(&state.vcpu_events) {
            let reason: String = format!("failed setting vcpu_events (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Mark vCPU as online.
        self.online = true;

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
    let fam_ref: &T = wrapper.as_fam_struct_ref();
    let total_size: usize = mem::size_of::<T>() + mem::size_of_val(fam_ref.as_slice());
    // SAFETY: FamStructWrapper guarantees that the header and entries are contiguous in memory.
    // The total_size accounts for the header plus all FAM entries.
    let raw_bytes: &[u8] =
        unsafe { slice::from_raw_parts((fam_ref as *const T).cast::<u8>(), total_size) };
    raw_bytes.to_vec()
}

///
/// # Description
///
/// Deserializes a CPUID table from a byte vector produced by [`serialize_fam_struct`].
///
/// # Parameters
///
/// - `bytes`: Byte vector containing the serialized `kvm_cpuid2` header followed by entries.
///
/// # Returns
///
/// A reconstructed [`CpuId`] on success, or an error if the data is malformed.
///
fn deserialize_cpuid(bytes: &[u8]) -> Result<CpuId> {
    let header_size: usize = mem::size_of::<kvm_cpuid2>();
    let entry_size: usize = mem::size_of::<kvm_cpuid_entry2>();

    if bytes.len() < header_size {
        let reason: &str = "cpuid data too short for header";
        error!("deserialize_cpuid(): {reason}");
        anyhow::bail!(reason)
    }

    // Read nent from the raw kvm_cpuid2 header (first 4 bytes, native endian).
    let nent: usize = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;

    // Cap nent to prevent pathological allocations from malformed snapshots.
    if nent > KVM_MAX_CPUID_ENTRIES {
        let reason: String = format!(
            "cpuid entry count {} exceeds KVM_MAX_CPUID_ENTRIES ({})",
            nent, KVM_MAX_CPUID_ENTRIES
        );
        error!("deserialize_cpuid(): {reason}");
        anyhow::bail!(reason)
    }

    let expected_size: usize = nent
        .checked_mul(entry_size)
        .and_then(|v| v.checked_add(header_size))
        .ok_or_else(|| anyhow::anyhow!("CPUID data size computation overflowed (nent={nent})"))?;
    if bytes.len() < expected_size {
        let reason: String = format!(
            "cpuid data size mismatch: expected at least {expected_size}, got {}",
            bytes.len()
        );
        error!("deserialize_cpuid(): {reason}");
        anyhow::bail!(reason)
    }

    let mut cpuid: CpuId = match CpuId::new(nent) {
        Ok(v) => v,
        Err(e) => {
            let reason: String = format!("failed creating CpuId (error={e:?})");
            error!("deserialize_cpuid(): {reason}");
            anyhow::bail!(reason)
        },
    };

    // Deserialize entries using unaligned reads into aligned CpuId storage.
    let entries_bytes: &[u8] = &bytes[header_size..header_size + nent * entry_size];

    for (i, chunk) in entries_bytes.chunks_exact(entry_size).enumerate() {
        // SAFETY:
        // - `chunk` has length exactly `entry_size` (by `chunks_exact`).
        // - We interpret the chunk as a `kvm_cpuid_entry2` via an unaligned read,
        //   which is allowed even if the underlying pointer is not properly aligned.
        // - The resulting value is then stored into `cpuid.as_mut_slice()[i]`,
        //   which is properly aligned for `kvm_cpuid_entry2`.
        let src: *const kvm_cpuid_entry2 = chunk.as_ptr().cast();
        let entry: kvm_cpuid_entry2 = unsafe { core::ptr::read_unaligned(src) };
        cpuid.as_mut_slice()[i] = entry;
    }

    Ok(cpuid)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use ::anyhow::Result as AnyResult;

    /// Creates a minimal KVM VM with a fully initialized `VirtualProcessor` for testing.
    /// Returns the `Kvm` and `VmFd` handles alongside the `VirtualProcessor` so that the KVM
    /// file descriptors remain open for the lifetime of the test.
    fn create_test_vcpu() -> AnyResult<(Kvm, VmFd, VirtualProcessor)> {
        let mut kvm: Kvm = Kvm::new().expect("failed to open /dev/kvm");
        let mut vm: VmFd = kvm.create_vm().expect("failed to create VM");
        // Create an in-kernel IRQ chip — required for LAPIC save/restore.
        vm.create_irq_chip().expect("failed to create IRQ chip");
        let vcpu: VirtualProcessor =
            VirtualProcessor::new(&mut kvm, &mut vm, 0).expect("failed to create VirtualProcessor");
        Ok((kvm, vm, vcpu))
    }

    /// Verifies that `save_state` produces a non-empty `VirtualProcessorState` with populated
    /// register fields and serializable CPUID/MSR/FPU byte vectors.
    #[test]
    fn save_state_produces_valid_snapshot() -> AnyResult<()> {
        let (kvm, _vm, vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

        let state: VirtualProcessorState = vcpu.save_state(&kvm).expect("save_state failed");

        // CPUID bytes must contain at least the kvm_cpuid2 header.
        assert!(
            state.cpuid.len() >= mem::size_of::<kvm_cpuid2>(),
            "CPUID snapshot too short (len={})",
            state.cpuid.len()
        );

        // TSC frequency must be non-zero on any modern host.
        assert!(state.tsc_khz > 0, "TSC frequency should be non-zero");

        // The state must be serializable (snapshot pipeline uses serde_cbor).
        let encoded: Vec<u8> =
            ::serde_cbor::to_vec(&state).expect("VirtualProcessorState should be serializable");
        assert!(!encoded.is_empty(), "serialized VirtualProcessorState should not be empty");

        Ok(())
    }

    /// MSR indices of volatile registers whose values may change between consecutive
    /// reads even without guest execution (hardware counters, timers, energy meters).
    /// These are excluded from the byte-for-byte round-trip comparison.
    const VOLATILE_MSR_INDICES: &[u32] = &[
        0x0010, // IA32_TSC
        0x0034, // SMI_COUNT
        0x00E7, // IA32_MPERF
        0x00E8, // IA32_APERF
        0x0198, // IA32_PERF_STATUS
        0x0611, // PKG_ENERGY_STATUS
        0x0613, // PKG_PERF_STATUS
        0x0619, // DRAM_ENERGY_STATUS
        0x061B, // DRAM_PERF_STATUS
        0x06E0, // IA32_TSC_DEADLINE
    ];

    /// Zeroes out the `data` field of every MSR entry whose index is in
    /// [`VOLATILE_MSR_INDICES`]. This allows two snapshots to be compared
    /// byte-for-byte even when volatile MSR values have changed between reads.
    fn zero_volatile_msr_values(state: &mut msr::MsrsState) {
        let header_size: usize = mem::size_of::<::kvm_bindings::kvm_msrs>();
        let entry_size: usize = mem::size_of::<kvm_msr_entry>();

        if state.bytes.len() < header_size {
            return;
        }

        let nmsrs: usize = u32::from_ne_bytes([
            state.bytes[0],
            state.bytes[1],
            state.bytes[2],
            state.bytes[3],
        ]) as usize;

        for i in 0..nmsrs {
            let entry_offset: usize = header_size + i * entry_size;
            if entry_offset + entry_size > state.bytes.len() {
                break;
            }
            // kvm_msr_entry layout: index (u32), reserved (u32), data (u64).
            let index: u32 = u32::from_ne_bytes([
                state.bytes[entry_offset],
                state.bytes[entry_offset + 1],
                state.bytes[entry_offset + 2],
                state.bytes[entry_offset + 3],
            ]);
            if VOLATILE_MSR_INDICES.contains(&index) {
                // Zero out the data field (bytes 8..16 within the entry).
                let data_offset: usize = entry_offset + 8;
                state.bytes[data_offset..data_offset + 8].fill(0);
            }
        }
    }

    /// Verifies that a save → load → save round trip produces identical vCPU state.
    #[test]
    fn save_load_round_trip() -> AnyResult<()> {
        let (kvm, _vm, mut vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

        // Save the initial state.
        let mut state_before: VirtualProcessorState =
            vcpu.save_state(&kvm).expect("first save_state failed");

        // Load it back.
        vcpu.load_state(&state_before).expect("load_state failed");
        assert!(vcpu.is_online(), "vCPU should be online after load_state");

        // Save again and compare the two snapshots.
        let mut state_after: VirtualProcessorState =
            vcpu.save_state(&kvm).expect("second save_state failed");

        // Zero out volatile MSR values (e.g. IA32_TSC, IA32_MPERF) before comparing,
        // as they are updated by hardware/KVM between consecutive reads.
        zero_volatile_msr_values(&mut state_before.msrs_state);
        zero_volatile_msr_values(&mut state_after.msrs_state);

        // Serialize both snapshots and compare bytes — this covers all fields including
        // the opaque CPUID, FPU, and MSR byte vectors (with volatile MSRs neutralized).
        let bytes_before: Vec<u8> =
            ::serde_cbor::to_vec(&state_before).expect("serialization of state_before failed");
        let bytes_after: Vec<u8> =
            ::serde_cbor::to_vec(&state_after).expect("serialization of state_after failed");
        assert_eq!(
            bytes_before, bytes_after,
            "vCPU state should be identical after a save-load-save round trip"
        );

        Ok(())
    }

    /// Verifies that `deserialize_cpuid` rejects a byte vector that is too short for the header.
    #[test]
    fn deserialize_cpuid_rejects_truncated_header() {
        let short_data: Vec<u8> = vec![0u8; 4];
        let result: Result<CpuId> = deserialize_cpuid(&short_data);
        assert!(result.is_err(), "deserialize_cpuid should reject truncated header");
    }

    /// Verifies that `deserialize_cpuid` rejects a header whose `nent` field implies more entries
    /// than the byte vector contains.
    #[test]
    fn deserialize_cpuid_rejects_truncated_entries() {
        // Create a header with nent = 10 but provide no entry bytes.
        let header_size: usize = mem::size_of::<kvm_cpuid2>();
        let mut data: Vec<u8> = vec![0u8; header_size];
        // Write nent = 10 at offset 0 (native endian).
        let nent_bytes: [u8; 4] = 10u32.to_ne_bytes();
        data[..4].copy_from_slice(&nent_bytes);

        let result: Result<CpuId> = deserialize_cpuid(&data);
        assert!(result.is_err(), "deserialize_cpuid should reject data with insufficient entries");
    }

    /// Verifies that the CPUID serialize → deserialize round trip preserves all entries.
    #[test]
    fn cpuid_serialize_deserialize_round_trip() -> AnyResult<()> {
        let (_kvm, _vm, vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

        let original: FamStructWrapper<kvm_cpuid2> = vcpu
            .fd
            .get_cpuid2(KVM_MAX_CPUID_ENTRIES)
            .expect("get_cpuid2 failed");

        let serialized: Vec<u8> = serialize_fam_struct(&original);
        let restored: CpuId = deserialize_cpuid(&serialized).expect("deserialize_cpuid failed");

        assert_eq!(
            original.as_slice().len(),
            restored.as_slice().len(),
            "CPUID entry count mismatch"
        );

        for (i, (orig, rest)) in original
            .as_slice()
            .iter()
            .zip(restored.as_slice().iter())
            .enumerate()
        {
            assert_eq!(orig.function, rest.function, "CPUID entry {i}: function mismatch");
            assert_eq!(orig.index, rest.index, "CPUID entry {i}: index mismatch");
            assert_eq!(orig.eax, rest.eax, "CPUID entry {i}: eax mismatch");
            assert_eq!(orig.ebx, rest.ebx, "CPUID entry {i}: ebx mismatch");
            assert_eq!(orig.ecx, rest.ecx, "CPUID entry {i}: ecx mismatch");
            assert_eq!(orig.edx, rest.edx, "CPUID entry {i}: edx mismatch");
        }

        Ok(())
    }

    /// LAPIC register array offset of the LVT timer entry.
    const LAPIC_LVT_TIMER_OFFSET: usize = 0x320;

    /// LVT timer value placing the timer in TSC-deadline mode (bits 17:18 = 0b10) with vector
    /// `0x40` and the mask bit cleared. The mode bits in this register are the canonical signal
    /// that `KVM_SET_LAPIC` was actually applied to the in-kernel APIC.
    const LVT_TIMER_TSC_DEADLINE_MODE: u32 = 0x0004_0040;

    /// Reads the four LVT-timer bytes back from the in-kernel LAPIC.
    fn read_lvt_timer(fd: &::kvm_ioctls::VcpuFd) -> u32 {
        let lapic: ::kvm_bindings::kvm_lapic_state = fd.get_lapic().expect("get_lapic failed");
        let bytes: [u8; 4] = [
            lapic.regs[LAPIC_LVT_TIMER_OFFSET] as u8,
            lapic.regs[LAPIC_LVT_TIMER_OFFSET + 1] as u8,
            lapic.regs[LAPIC_LVT_TIMER_OFFSET + 2] as u8,
            lapic.regs[LAPIC_LVT_TIMER_OFFSET + 3] as u8,
        ];
        u32::from_ne_bytes(bytes)
    }

    /// Contract test: `load_state` must invoke `KVM_SET_LAPIC` as part of the restore pipeline,
    /// and the LAPIC bytes carried by the snapshot must survive every subsequent vCPU ioctl in
    /// the sequence. This guards against accidental removal/reordering of `set_lapic` and against
    /// any future ioctl in `load_state` overwriting LAPIC state.
    ///
    /// Note: this test does NOT specifically detect a `SET_MSRS`-before-`SET_LAPIC` ordering
    /// regression — its symptom (spurious early timer IRQ on resume) only surfaces while the
    /// guest is executing. The integration/benchmark-level coverage in
    /// `nanvix-bench -benchmark snapshot-restore` is the authoritative regression guard for it.
    #[test]
    fn load_state_applies_lapic_after_other_vcpu_ioctls() -> AnyResult<()> {
        let (kvm, _vm, mut vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

        let mut state: VirtualProcessorState = vcpu.save_state(&kvm).expect("save_state failed");

        // Stamp the LVT timer register with a distinctive value in the snapshot.
        let lvt_bytes: [u8; 4] = LVT_TIMER_TSC_DEADLINE_MODE.to_ne_bytes();
        for (i, b) in lvt_bytes.iter().enumerate() {
            state.lapic.regs[LAPIC_LVT_TIMER_OFFSET + i] = (*b).cast_signed();
        }

        // Sanity-check: ensure the LVT is *not* already in this configuration on a fresh vCPU.
        // Otherwise the test cannot tell whether `load_state` actually wrote the LAPIC.
        let lvt_before: u32 = read_lvt_timer(&vcpu.fd);
        assert_ne!(
            lvt_before, LVT_TIMER_TSC_DEADLINE_MODE,
            "test setup is invalid: fresh vCPU already has the fingerprint LVT value"
        );

        vcpu.load_state(&state).expect("load_state failed");

        let lvt_after: u32 = read_lvt_timer(&vcpu.fd);
        assert_eq!(
            lvt_after, LVT_TIMER_TSC_DEADLINE_MODE,
            "LAPIC LVT-timer fingerprint not present after load_state — KVM_SET_LAPIC was either \
             skipped or overwritten by a subsequent ioctl"
        );

        Ok(())
    }
}
