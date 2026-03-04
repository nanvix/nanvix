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
#[cfg(feature = "x86_64")]
use crate::vmm::kvm::vmem::VirtualMemory;
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

/// IA32_EFER MSR — Extended Feature Enable Register.
#[cfg(feature = "x86_64")]
const MSR_IA32_EFER: u32 = 0xC0000080;

/// EFER.LME — Long Mode Enable.
#[cfg(feature = "x86_64")]
const EFER_LME: u64 = 1 << 8;

/// EFER.LMA — Long Mode Active.
#[cfg(feature = "x86_64")]
const EFER_LMA: u64 = 1 << 10;

/// EFER.SCE — System Call Extensions.
#[cfg(feature = "x86_64")]
const EFER_SCE: u64 = 1 << 0;

/// Guest physical address where the GDT is placed.
#[cfg(feature = "x86_64")]
const GDT_GPA: u64 = 0x2000;

/// Guest physical address where the PML4 page table is placed.
#[cfg(feature = "x86_64")]
const PML4_GPA: u64 = 0x3000;

/// Guest physical address where the PDPT page table is placed.
#[cfg(feature = "x86_64")]
const PDPT_GPA: u64 = 0x4000;

/// Guest physical address where the PD page table is placed.
#[cfg(feature = "x86_64")]
const PD_GPA: u64 = 0x5000;

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

    ///
    /// # Description
    ///
    /// Resets the virtual processor into x86_64 long mode.
    ///
    /// This sets up identity-mapped page tables (using 2MB huge pages), a minimal GDT,
    /// and configures segment registers, control registers, and the EFER MSR for 64-bit
    /// long mode execution.
    ///
    /// # Parameters
    ///
    /// - `rip`: Entry point address.
    /// - `rax`: Value to set the `rax` register.
    /// - `rbx`: Value to set the `rbx` register.
    /// - `vmem`: Guest virtual memory (for writing page tables and GDT).
    /// - `memory_size`: Total guest physical memory size in bytes.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    #[cfg(feature = "x86_64")]
    pub fn reset_long_mode(
        &mut self,
        rip: u64,
        rax: u64,
        rbx: u64,
        vmem: &mut VirtualMemory,
        memory_size: usize,
    ) -> Result<()> {
        trace!(
            "reset_long_mode(): rip={rip:#010x}, rax={rax:#010x}, rbx={rbx:#010x}, \
             memory_size={memory_size:#010x}"
        );

        // Write GDT into guest memory.
        Self::write_gdt(vmem)?;

        // Write identity-mapped page tables into guest memory.
        Self::write_page_tables(vmem, memory_size)?;

        // Set up EFER MSR for long mode.
        self.setup_efer()?;

        // Configure system registers for long mode.
        let mut vcpu_sregs: kvm_sregs = self.fd.get_sregs()?;

        // 64-bit code segment (selector 0x08).
        vcpu_sregs.cs.base = 0;
        vcpu_sregs.cs.limit = 0xFFFF_FFFF;
        vcpu_sregs.cs.selector = 0x08;
        vcpu_sregs.cs.type_ = 0x0B; // Execute/Read, accessed.
        vcpu_sregs.cs.present = 1;
        vcpu_sregs.cs.dpl = 0;
        vcpu_sregs.cs.db = 0; // Must be 0 for 64-bit code segment.
        vcpu_sregs.cs.s = 1; // Code/data segment.
        vcpu_sregs.cs.l = 1; // Long mode flag.
        vcpu_sregs.cs.g = 1; // Page granularity.

        // 64-bit data segments (selector 0x10).
        let data_seg = kvm_bindings::kvm_segment {
            base: 0,
            limit: 0xFFFF_FFFF,
            selector: 0x10,
            type_: 0x03, // Read/Write, accessed.
            present: 1,
            dpl: 0,
            db: 1,
            s: 1,
            l: 0,
            g: 1,
            avl: 0,
            unusable: 0,
            padding: 0,
        };
        vcpu_sregs.ds = data_seg;
        vcpu_sregs.es = data_seg;
        vcpu_sregs.ss = data_seg;
        vcpu_sregs.fs = data_seg;
        vcpu_sregs.gs = data_seg;

        // CR0: PE=1, PG=1, ET=1, NE=1, WP=1, MP=1.
        vcpu_sregs.cr0 = (1 << 0)  // PE — Protection Enable
            | (1 << 1)             // MP — Monitor Coprocessor
            | (1 << 4)             // ET — Extension Type
            | (1 << 5)             // NE — Numeric Error
            | (1 << 16)            // WP — Write Protect
            | (1 << 31);           // PG — Paging

        // CR3: Physical address of PML4.
        vcpu_sregs.cr3 = PML4_GPA;

        // CR4: PAE=1, OSFXSR=1, OSXMMEXCPT=1.
        vcpu_sregs.cr4 = (1 << 5)  // PAE — Physical Address Extension (required for long mode)
            | (1 << 9)             // OSFXSR — FXSAVE/FXRSTOR support
            | (1 << 10);           // OSXMMEXCPT — Unmasked SIMD FP Exceptions support

        // GDT register: point to GDT in guest memory (3 entries × 8 bytes = 24 bytes).
        vcpu_sregs.gdt.base = GDT_GPA;
        vcpu_sregs.gdt.limit = 3 * 8 - 1;

        // IDT: empty for now (the kernel will set up its own).
        vcpu_sregs.idt.base = 0;
        vcpu_sregs.idt.limit = 0;

        self.fd.set_sregs(&vcpu_sregs)?;

        // Set general purpose registers.
        let mut vcpu_regs: kvm_regs = self.fd.get_regs()?;
        vcpu_regs.rip = rip;
        vcpu_regs.rax = rax;
        vcpu_regs.rbx = rbx;
        vcpu_regs.rsp = 0; // Kernel will set up its own stack.
        vcpu_regs.rflags = RFLAGS_INTERRUPT_ENABLE;
        self.fd.set_regs(&vcpu_regs)?;

        // Processor is now online.
        self.online = true;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Writes a minimal GDT for 64-bit long mode into guest memory.
    ///
    /// Layout:
    /// - Entry 0 (0x00): Null descriptor
    /// - Entry 1 (0x08): 64-bit code segment
    /// - Entry 2 (0x10): 64-bit data segment
    ///
    #[cfg(feature = "x86_64")]
    fn write_gdt(vmem: &mut VirtualMemory) -> Result<()> {
        trace!("write_gdt(): gdt_gpa={GDT_GPA:#010x}");

        let mut gdt: [u64; 3] = [0u64; 3];

        // Entry 0: Null descriptor.
        gdt[0] = 0;

        // Entry 1 (selector 0x08): 64-bit code segment.
        // Access byte: 0x9A (P=1, DPL=0, S=1, Type=0xA — Execute/Read)
        // Flags: 0xA (L=1 for 64-bit, G=1 for page granularity)
        // Limit 0xFFFFF with G=1 → 4GB.
        gdt[1] = Self::encode_gdt_entry(0, 0xFFFFF, 0x9A, 0xA);

        // Entry 2 (selector 0x10): 64-bit data segment.
        // Access byte: 0x92 (P=1, DPL=0, S=1, Type=0x2 — Read/Write)
        // Flags: 0xC (D/B=1 for 32-bit operand size, G=1 for page granularity)
        gdt[2] = Self::encode_gdt_entry(0, 0xFFFFF, 0x92, 0xC);

        let gdt_bytes: &[u8] = unsafe {
            slice::from_raw_parts(gdt.as_ptr().cast::<u8>(), gdt.len() * mem::size_of::<u64>())
        };

        vmem.write_bytes(GDT_GPA, gdt_bytes)
    }

    /// Encodes a single 8-byte GDT entry from base, limit, access byte, and flags nibble.
    #[cfg(feature = "x86_64")]
    fn encode_gdt_entry(base: u32, limit: u32, access: u8, flags: u8) -> u64 {
        let mut entry: u64 = 0;

        // Limit bits 0-15.
        entry |= (limit & 0xFFFF) as u64;
        // Base bits 0-15.
        entry |= ((base & 0xFFFF) as u64) << 16;
        // Base bits 16-23.
        entry |= (((base >> 16) & 0xFF) as u64) << 32;
        // Access byte.
        entry |= (access as u64) << 40;
        // Limit bits 16-19.
        entry |= (((limit >> 16) & 0x0F) as u64) << 48;
        // Flags nibble (bits 52-55).
        entry |= ((flags & 0x0F) as u64) << 52;
        // Base bits 24-31.
        entry |= (((base >> 24) & 0xFF) as u64) << 56;

        entry
    }

    ///
    /// # Description
    ///
    /// Writes identity-mapped page tables for x86_64 long mode into guest memory.
    ///
    /// Uses 2MB huge pages for simplicity. Creates PML4 → PDPT → PD hierarchy.
    ///
    #[cfg(feature = "x86_64")]
    fn write_page_tables(vmem: &mut VirtualMemory, memory_size: usize) -> Result<()> {
        trace!("write_page_tables(): memory_size={memory_size:#010x}");

        const PAGE_TABLE_SIZE: usize = 4096;
        const HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024; // 2MB

        // Zero out the page table pages first.
        let zeros: [u8; PAGE_TABLE_SIZE] = [0u8; PAGE_TABLE_SIZE];
        vmem.write_bytes(PML4_GPA, &zeros)?;
        vmem.write_bytes(PDPT_GPA, &zeros)?;
        vmem.write_bytes(PD_GPA, &zeros)?;

        // PML4[0] → PDPT (present, writable).
        let pml4_entry: u64 = PDPT_GPA | 0x03; // Present + Writable
        vmem.write_bytes(PML4_GPA, &pml4_entry.to_le_bytes())?;

        // PDPT[0] → PD (present, writable).
        let pdpt_entry: u64 = PD_GPA | 0x03; // Present + Writable
        vmem.write_bytes(PDPT_GPA, &pdpt_entry.to_le_bytes())?;

        // PD entries: identity-map using 2MB huge pages.
        let num_huge_pages: usize = (memory_size + HUGE_PAGE_SIZE - 1) / HUGE_PAGE_SIZE;
        // Cap at 512 entries (1GB maximum with a single PD).
        let num_entries: usize = if num_huge_pages > 512 { 512 } else { num_huge_pages };

        for i in 0..num_entries {
            let phys_addr: u64 = (i * HUGE_PAGE_SIZE) as u64;
            // Present + Writable + PS (Page Size, indicates 2MB page).
            let pd_entry: u64 = phys_addr | 0x83;
            let offset: u64 = PD_GPA + (i as u64) * 8;
            vmem.write_bytes(offset, &pd_entry.to_le_bytes())?;
        }

        trace!(
            "write_page_tables(): mapped {} x 2MB huge pages ({} bytes total)",
            num_entries,
            num_entries * HUGE_PAGE_SIZE
        );

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Sets up the EFER MSR for x86_64 long mode.
    ///
    #[cfg(feature = "x86_64")]
    fn setup_efer(&mut self) -> Result<()> {
        trace!("setup_efer()");

        let efer_value: u64 = EFER_LME | EFER_LMA | EFER_SCE;

        let msr_entries: Vec<kvm_msr_entry> = vec![kvm_msr_entry {
            index: MSR_IA32_EFER,
            data: efer_value,
            ..Default::default()
        }];

        let msrs: ::kvm_bindings::Msrs = match ::kvm_bindings::Msrs::from_entries(&msr_entries) {
            Ok(v) => v,
            Err(e) => {
                let reason: String = format!("failed to create MSRs for EFER (error={e:?})");
                error!("setup_efer(): {reason}");
                anyhow::bail!(reason)
            },
        };

        match self.fd.set_msrs(&msrs) {
            Ok(written) if written == msr_entries.len() => {
                trace!("setup_efer(): EFER MSR set (value={efer_value:#018x})");
                Ok(())
            },
            Ok(written) => {
                let reason: String = format!(
                    "failed to set all EFER MSRs: written {written} of {} entries",
                    msr_entries.len()
                );
                error!("setup_efer(): {reason}");
                anyhow::bail!(reason)
            },
            Err(e) => {
                let reason: String = format!("failed to set EFER MSR (error={e:?})");
                error!("setup_efer(): {reason}");
                anyhow::bail!(reason)
            },
        }
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
        // Ordering requirements between `kvm_set` calls:
        // https://github.com/firecracker-microvm/firecracker/blob/f0691f8253d4bde225b9f70ecabf39b7ad796935/src/vmm/src/arch/x86_64/vcpu.rs#L556
        trace!("load_state()");

        // Restore system registers first (some MSRs depend on sregs).
        if let Err(e) = self.fd.set_sregs(&state.sregs) {
            let reason: String = format!("failed setting sregs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore CPUID.
        let cpuid: CpuId = deserialize_cpuid(&state.cpuid)?;
        if let Err(e) = self.fd.set_cpuid2(&cpuid) {
            let reason: String = format!("failed setting cpuid (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore MSRs (after sregs).
        if let Err(e) = self.msrs.load_state(&self.fd, &state.msrs_state) {
            let reason: String = format!("failed setting msrs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore general purpose registers.
        if let Err(e) = self.fd.set_regs(&state.regs) {
            let reason: String = format!("failed setting regs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore LAPIC.
        if let Err(e) = self.fd.set_lapic(&state.lapic) {
            let reason: String = format!("failed setting lapic (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore TSC frequency.
        if let Err(e) = self.fd.set_tsc_khz(state.tsc_khz) {
            let reason: String = format!("failed setting tsc_khz (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore debug registers.
        if let Err(e) = self.fd.set_debug_regs(&state.debugregs) {
            let reason: String = format!("failed setting debugregs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore extended control registers.
        if let Err(e) = self.fd.set_xcrs(&state.xcrs) {
            let reason: String = format!("failed setting xcrs (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore FPU/XSAVE state.
        if let Err(e) = self.fpu.load_state(&self.fd, &state.fpu_ext) {
            let reason: String = format!("failed setting fpu state (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore vCPU events.
        if let Err(e) = self.fd.set_vcpu_events(&state.vcpu_events) {
            let reason: String = format!("failed setting vcpu_events (error={e:?})");
            error!("load_state(): {reason}");
            anyhow::bail!(reason)
        }

        // Restore MP state last (setting it to runnable starts execution).
        if let Err(e) = self.fd.set_mp_state(state.mp_state) {
            let reason: String = format!("failed setting mp_state (error={e:?})");
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

    let expected_size: usize = header_size + nent * entry_size;
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

    /// Verifies that a save → load → save round trip produces identical vCPU state.
    #[test]
    fn save_load_round_trip() -> AnyResult<()> {
        let (kvm, _vm, mut vcpu): (Kvm, VmFd, VirtualProcessor) = create_test_vcpu()?;

        // Save the initial state.
        let state_before: VirtualProcessorState =
            vcpu.save_state(&kvm).expect("first save_state failed");

        // Load it back.
        vcpu.load_state(&state_before).expect("load_state failed");
        assert!(vcpu.is_online(), "vCPU should be online after load_state");

        // Save again and compare the two snapshots field-by-field.
        let state_after: VirtualProcessorState =
            vcpu.save_state(&kvm).expect("second save_state failed");

        // Serialize both snapshots and compare bytes — this covers all fields including
        // the opaque CPUID, FPU, and MSR byte vectors.
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
}
