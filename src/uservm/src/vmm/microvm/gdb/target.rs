// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Implementation of the `gdbstub::target::Target` trait for the Nanvix microvm.
//!
//! `NanvixTarget` wraps `Arc<Mutex<...>>` handles to the vCPU and virtual memory, translating
//! GDB protocol operations into KVM ioctls and direct memory access.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::{
    InteriorMicroVmHandle,
    kvm::{
        vcpu::VirtualProcessor,
        vmem::VirtualMemory,
    },
};
use ::anyhow::Result;
use ::gdbstub::{
    arch::Arch,
    target::{
        Target,
        TargetError,
        TargetResult,
        ext::{
            base::singlethread::{
                SingleThreadBase,
                SingleThreadResume,
                SingleThreadResumeOps,
                SingleThreadSingleStep,
                SingleThreadSingleStepOps,
            },
            breakpoints::{
                Breakpoints,
                BreakpointsOps,
                SwBreakpoint,
                SwBreakpointOps,
            },
        },
    },
};
use ::gdbstub_arch::x86::X86_64_SSE;
use ::kvm_bindings::{
    KVM_GUESTDBG_ENABLE,
    KVM_GUESTDBG_SINGLESTEP,
    KVM_GUESTDBG_USE_SW_BP,
    kvm_guest_debug,
};
use ::log::{
    error,
    trace,
};
use ::std::{
    collections::HashMap,
    sync::Arc,
};
use ::tokio::sync::Mutex;

//==================================================================================================
// Types
//==================================================================================================

type GdbTargetError = &'static str;

/// The x86_64 `INT3` opcode used for software breakpoints.
pub(super) const INT3: u8 = 0xCC;

//==================================================================================================
// NanvixTarget
//==================================================================================================

/// GDB target wrapping the microvm vCPU and virtual memory.
///
/// Uses `Arc<Mutex<...>>` handles so the target can be `'static` as required by `gdbstub`'s
/// `BlockingEventLoop`. Locks are acquired per-operation and released immediately.
pub struct NanvixTarget {
    pub vcpu: Arc<Mutex<VirtualProcessor>>,
    pub vmem: Arc<Mutex<VirtualMemory>>,
    /// VMM interior handle providing access to the I/O emulator.
    pub inner: Arc<Mutex<InteriorMicroVmHandle>>,
    /// Maps breakpoint address to the original byte that was overwritten by `INT3`.
    pub sw_breakpoints: HashMap<u64, u8>,
    /// Whether the next resume should single-step.
    pub single_step: bool,
    /// Address of a software breakpoint that needs step-over on the next resume.
    pub pending_bp_addr: Option<u64>,
}

impl Target for NanvixTarget {
    type Arch = X86_64_SSE;
    type Error = GdbTargetError;

    #[inline(always)]
    fn base_ops(&mut self) -> gdbstub::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        gdbstub::target::ext::base::BaseOps::SingleThread(self)
    }

    #[inline(always)]
    fn support_breakpoints(&mut self) -> Option<BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

//==================================================================================================
// SingleThreadBase
//==================================================================================================

impl SingleThreadBase for NanvixTarget {
    fn read_registers(
        &mut self,
        regs: &mut <X86_64_SSE as Arch>::Registers,
    ) -> TargetResult<(), Self> {
        let vcpu = self.vcpu.blocking_lock();
        let kvm_regs = vcpu.get_regs().map_err(|e| {
            error!("gdb: failed to read registers: {e:?}");
            TargetError::Fatal("get_regs failed")
        })?;

        let kvm_sregs = vcpu.get_sregs().map_err(|e| {
            error!("gdb: failed to read segment registers: {e:?}");
            TargetError::Fatal("get_sregs failed")
        })?;

        // Map KVM registers to gdbstub x86_64 register file.
        regs.regs = [
            kvm_regs.rax,
            kvm_regs.rbx,
            kvm_regs.rcx,
            kvm_regs.rdx,
            kvm_regs.rsi,
            kvm_regs.rdi,
            kvm_regs.rbp,
            kvm_regs.rsp,
            kvm_regs.r8,
            kvm_regs.r9,
            kvm_regs.r10,
            kvm_regs.r11,
            kvm_regs.r12,
            kvm_regs.r13,
            kvm_regs.r14,
            kvm_regs.r15,
        ];
        regs.rip = kvm_regs.rip;
        regs.eflags = u32::try_from(kvm_regs.rflags & 0xFFFF_FFFF).unwrap_or(0);

        // Segment registers.
        regs.segments = gdbstub_arch::x86::reg::X86SegmentRegs {
            cs: u32::from(kvm_sregs.cs.selector),
            ss: u32::from(kvm_sregs.ss.selector),
            ds: u32::from(kvm_sregs.ds.selector),
            es: u32::from(kvm_sregs.es.selector),
            fs: u32::from(kvm_sregs.fs.selector),
            gs: u32::from(kvm_sregs.gs.selector),
        };

        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &<X86_64_SSE as Arch>::Registers,
    ) -> TargetResult<(), Self> {
        let vcpu = self.vcpu.blocking_lock();
        let mut kvm_regs = vcpu.get_regs().map_err(|e| {
            error!("gdb: failed to read registers for write: {e:?}");
            TargetError::Fatal("get_regs failed")
        })?;

        kvm_regs.rax = regs.regs[0];
        kvm_regs.rbx = regs.regs[1];
        kvm_regs.rcx = regs.regs[2];
        kvm_regs.rdx = regs.regs[3];
        kvm_regs.rsi = regs.regs[4];
        kvm_regs.rdi = regs.regs[5];
        kvm_regs.rbp = regs.regs[6];
        kvm_regs.rsp = regs.regs[7];
        kvm_regs.r8 = regs.regs[8];
        kvm_regs.r9 = regs.regs[9];
        kvm_regs.r10 = regs.regs[10];
        kvm_regs.r11 = regs.regs[11];
        kvm_regs.r12 = regs.regs[12];
        kvm_regs.r13 = regs.regs[13];
        kvm_regs.r14 = regs.regs[14];
        kvm_regs.r15 = regs.regs[15];
        kvm_regs.rip = regs.rip;
        kvm_regs.rflags = u64::from(regs.eflags);

        vcpu.set_regs(&kvm_regs).map_err(|e| {
            error!("gdb: failed to write registers: {e:?}");
            TargetError::Fatal("set_regs failed")
        })
    }

    fn read_addrs(
        &mut self,
        start_addr: <X86_64_SSE as Arch>::Usize,
        data: &mut [u8],
    ) -> TargetResult<usize, Self> {
        let vmem = self.vmem.blocking_lock();
        match vmem.read_bytes(start_addr, data) {
            Ok(()) => Ok(data.len()),
            Err(e) => {
                trace!("gdb: memory read at {start_addr:#x} failed: {e:?}");
                Ok(0)
            },
        }
    }

    fn write_addrs(
        &mut self,
        start_addr: <X86_64_SSE as Arch>::Usize,
        data: &[u8],
    ) -> TargetResult<(), Self> {
        let mut vmem = self.vmem.blocking_lock();
        vmem.write_bytes(start_addr, data).map_err(|e| {
            error!("gdb: memory write at {start_addr:#x} failed: {e:?}");
            TargetError::Fatal("write_bytes failed")
        })
    }

    #[inline(always)]
    fn support_resume(&mut self) -> Option<SingleThreadResumeOps<'_, Self>> {
        Some(self)
    }
}

//==================================================================================================
// SingleThreadResume
//==================================================================================================

impl SingleThreadResume for NanvixTarget {
    fn resume(&mut self, _signal: Option<gdbstub::common::Signal>) -> Result<(), Self::Error> {
        self.single_step = false;
        Ok(())
    }

    #[inline(always)]
    fn support_single_step(&mut self) -> Option<SingleThreadSingleStepOps<'_, Self>> {
        Some(self)
    }
}

impl SingleThreadSingleStep for NanvixTarget {
    fn step(&mut self, _signal: Option<gdbstub::common::Signal>) -> Result<(), Self::Error> {
        self.single_step = true;
        Ok(())
    }
}

//==================================================================================================
// Breakpoints
//==================================================================================================

impl Breakpoints for NanvixTarget {
    #[inline(always)]
    fn support_sw_breakpoint(&mut self) -> Option<SwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl SwBreakpoint for NanvixTarget {
    fn add_sw_breakpoint(
        &mut self,
        addr: <X86_64_SSE as Arch>::Usize,
        _kind: <X86_64_SSE as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        // If the breakpoint already exists, return early to avoid overwriting the stored
        // original byte with INT3.
        if self.sw_breakpoints.contains_key(&addr) {
            return Ok(true);
        }

        let mut vmem = self.vmem.blocking_lock();

        // Read the original byte at the breakpoint address.
        let mut orig = [0u8; 1];
        match vmem.read_bytes(addr, &mut orig) {
            Ok(()) => {},
            Err(_) => return Ok(false),
        }

        // Write INT3 opcode.
        if let Err(e) = vmem.write_bytes(addr, &[INT3]) {
            error!("gdb: failed to write INT3 at {addr:#x}: {e:?}");
            return Ok(false);
        }

        self.sw_breakpoints.insert(addr, orig[0]);
        trace!("gdb: added software breakpoint at {addr:#x}");
        Ok(true)
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: <X86_64_SSE as Arch>::Usize,
        _kind: <X86_64_SSE as Arch>::BreakpointKind,
    ) -> TargetResult<bool, Self> {
        if let Some(orig_byte) = self.sw_breakpoints.remove(&addr) {
            let mut vmem = self.vmem.blocking_lock();
            if let Err(e) = vmem.write_bytes(addr, &[orig_byte]) {
                error!("gdb: failed to restore byte at {addr:#x}: {e:?}");
                return Ok(false);
            }
            trace!("gdb: removed software breakpoint at {addr:#x}");
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

//==================================================================================================
// Helper: Configure KVM Guest Debug
//==================================================================================================

/// Configures KVM guest debug mode on the vCPU.
///
/// Enables software breakpoints and optionally single-stepping.
pub fn configure_guest_debug(vcpu: &Arc<Mutex<VirtualProcessor>>, single_step: bool) -> Result<()> {
    let mut control: u32 = KVM_GUESTDBG_ENABLE | KVM_GUESTDBG_USE_SW_BP;
    if single_step {
        control |= KVM_GUESTDBG_SINGLESTEP;
    }

    let dbg = kvm_guest_debug {
        control,
        ..Default::default()
    };

    vcpu.blocking_lock().set_guest_debug(&dbg)
}
