// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::cpu::tss;
use ::arch::cpu::tss::Tss;
use ::core::arch::asm;
use ::sys::mm::{
    Address,
    VirtualAddress,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores the information about the execution context of a thread.
///
/// This struct matches the layout defined in `hooks.S` for x86_64.
/// Total size: 23 fields * 8 bytes = 184 bytes.
///
#[derive(Default)]
#[repr(C, packed)]
pub struct ContextInformation {
    rsp0: u64,
    cr3: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rbp: u64,
    rsi: u64,
    rdi: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
    err: u64,
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

// `ContextInformation` must be 184 bytes long. This must match low-level assembly dispatcher code.
::static_assert::assert_eq_size!(ContextInformation, 184);

//==================================================================================================
// Implementations
//==================================================================================================

impl ContextInformation {
    pub fn new(cr3: u64, rsp: u64, rsp0: u64) -> Self {
        Self {
            rsp0,
            cr3,
            rsp,
            ..Default::default()
        }
    }

    /// Returns the instruction pointer.
    #[allow(dead_code)]
    pub fn rip(&self) -> u64 {
        self.rip
    }

    /// Returns the stack pointer.
    pub fn rsp(&self) -> u64 {
        self.rsp
    }

    /// Sets the instruction pointer (for signal delivery).
    ///
    /// # Safety
    ///
    /// The struct is `repr(C, packed)`, so field writes may be unaligned.
    pub fn set_rip(&mut self, val: u64) {
        unsafe { core::ptr::addr_of_mut!(self.rip).write_unaligned(val) };
    }

    /// Sets the stack pointer (for signal delivery).
    ///
    /// # Safety
    ///
    /// The struct is `repr(C, packed)`, so field writes may be unaligned.
    pub fn set_rsp(&mut self, val: u64) {
        unsafe { core::ptr::addr_of_mut!(self.rsp).write_unaligned(val) };
    }

    /// Sets the first argument register (RDI on x86_64).
    ///
    /// # Safety
    ///
    /// The struct is `repr(C, packed)`, so field writes may be unaligned.
    pub fn set_rdi(&mut self, val: u64) {
        unsafe { core::ptr::addr_of_mut!(self.rdi).write_unaligned(val) };
    }

    ///
    /// # Description
    ///
    /// Switches to another execution context.
    ///
    /// # Parameters
    ///
    /// - `from`: Execution context to switch from.
    /// - `to`: Execution context to switch to.
    /// - `user_tda`: Optional base address for the user-space thread data area for the next
    ///   context.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it performs a context switch between two execution contexts.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - `from` and `to` point to valid execution contexts.
    /// - The processor is running with interrupts disabled.
    /// - The processor is running in privileged mode.
    ///
    /// # Notes
    ///
    /// This function does not return to the caller immediately. Instead, it switches to the `to`
    /// context. When the `from` context is switched back to, this function will return.
    ///
    pub unsafe fn switch(
        from: *mut ContextInformation,
        to: *mut ContextInformation,
        user_tda: Option<VirtualAddress>,
    ) {
        unsafe extern "C" {
            pub fn __context_switch(
                from: *mut ContextInformation,
                to: *mut ContextInformation,
                tss: *const Tss,
            );
        }

        // In x86_64, thread data area is set via the FS_BASE MSR rather than GDT-based
        // segment descriptors. Set the FS_BASE MSR to the TDA base address for the next
        // context, or clear it if there is no TDA.
        const IA32_FS_BASE: u32 = 0xC000_0100;
        let tda_value: u64 = match user_tda {
            Some(addr) => addr.into_raw_value() as u64,
            None => 0,
        };
        let eax: u32 = tda_value as u32;
        let edx: u32 = (tda_value >> 32) as u32;
        asm!(
            "wrmsr",
            in("ecx") IA32_FS_BASE,
            in("eax") eax,
            in("edx") edx,
            options(nomem, preserves_flags, nostack),
        );

        let tss: *const Tss = tss::get_curr();

        // Set CR0.TS flag to disable FPU/SSE instructions for the new thread.
        // This implements lazy FPU context switching. If the new thread attempts to use
        // FPU/SSE instructions, a #NM exception will be raised and handled appropriately.
        crate::hal::arch::set_task_switched();

        __context_switch(from, to, tss);
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl core::fmt::Debug for ContextInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // Copy fields to local variables to avoid unaligned access on packed struct.
        let rsp0: u64 = self.rsp0;
        let cr3: u64 = self.cr3;
        let r15: u64 = self.r15;
        let r14: u64 = self.r14;
        let r13: u64 = self.r13;
        let r12: u64 = self.r12;
        let r11: u64 = self.r11;
        let r10: u64 = self.r10;
        let r9: u64 = self.r9;
        let r8: u64 = self.r8;
        let rbp: u64 = self.rbp;
        let rsi: u64 = self.rsi;
        let rdi: u64 = self.rdi;
        let rdx: u64 = self.rdx;
        let rcx: u64 = self.rcx;
        let rbx: u64 = self.rbx;
        let rax: u64 = self.rax;
        let err: u64 = self.err;
        let rip: u64 = self.rip;
        let cs: u64 = self.cs;
        let rflags: u64 = self.rflags;
        let rsp: u64 = self.rsp;
        let ss: u64 = self.ss;

        write!(
            f,
            "rsp0={rsp0:#018x}, cr3={cr3:#018x}, r15={r15:#018x}, r14={r14:#018x}, \
             r13={r13:#018x}, r12={r12:#018x}, r11={r11:#018x}, r10={r10:#018x}, r9={r9:#018x}, \
             r8={r8:#018x}, rbp={rbp:#018x}, rsi={rsi:#018x}, rdi={rdi:#018x}, rdx={rdx:#018x}, \
             rcx={rcx:#018x}, rbx={rbx:#018x}, rax={rax:#018x}, err={err:#018x}, rip={rip:#018x}, \
             cs={cs:#018x}, rflags={rflags:#018x}, rsp={rsp:#018x}, ss={ss:#018x}",
        )
    }
}
