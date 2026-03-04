// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::cpu::tss::Tss;
use ::sys::mm::VirtualAddress;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores the information about the execution context of a thread (64-bit).
///
/// This layout must match the assembly code in hooks.S exactly.
///
#[derive(Default)]
#[repr(C, packed)]
pub struct ContextInformation {
    rsp0: u64,
    cr3: u64,
    gs: u64,
    fs: u64,
    es: u64,
    ds: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rdi: u64,
    rsi: u64,
    rbp: u64,
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

// Must match CONTEXT_SIZE in hooks.S: 27 * 8 = 216
::static_assert::assert_eq_size!(ContextInformation, 216);

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

    pub unsafe fn switch(
        from: *mut ContextInformation,
        to: *mut ContextInformation,
        _user_tda: Option<VirtualAddress>,
    ) {
        unsafe extern "C" {
            pub fn __context_switch(
                from: *mut ContextInformation,
                to: *mut ContextInformation,
                tss: *const Tss,
            );
        }

        let tss: *const Tss = super::tss::get_curr();

        crate::hal::arch::set_task_switched();

        __context_switch(from, to, tss);
    }
}

impl core::fmt::Debug for ContextInformation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let rsp0: u64 = self.rsp0;
        let cr3: u64 = self.cr3;
        let rip: u64 = self.rip;
        let rsp: u64 = self.rsp;
        let rflags: u64 = self.rflags;

        write!(
            f,
            "rsp0={rsp0:#018x}, cr3={cr3:#018x}, rip={rip:#018x}, \
             rsp={rsp:#018x}, rflags={rflags:#018x}",
        )
    }
}
