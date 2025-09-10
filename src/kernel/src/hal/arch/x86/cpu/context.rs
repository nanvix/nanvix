// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::{
    cpu::tss,
    mem::gdt::{
        Gdt,
        SegmentSelector,
    },
};
use ::arch::cpu::tss::Tss;
use ::sys::mm::VirtualAddress;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Stores the information about the execution context of a thread.
///
#[derive(Default)]
#[repr(C, packed)]
pub struct ContextInformation {
    esp0: u32,
    cr3: u32,
    gs: u32,
    fs: u32,
    es: u32,
    ds: u32,
    edi: u32,
    esi: u32,
    ebp: u32,
    edx: u32,
    ecx: u32,
    ebx: u32,
    eax: u32,
    err: u32,
    eip: u32,
    cs: u32,
    eflags: u32,
    esp: u32,
    ss: u32,
}

// `Context` must be 72 bytes long. This must match low-level assembly dispatcher code.
::static_assert::assert_eq_size!(ContextInformation, 76);

//==================================================================================================
// Implementations
//==================================================================================================

impl ContextInformation {
    pub fn new(cr3: u32, esp: u32, esp0: u32) -> Self {
        Self {
            esp0,
            cr3,
            esp,
            ..Default::default()
        }
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
    /// - `user_tda`: Optional base address for the user-space thread data area for the next context.
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

        // Set thread data area.
        if let Some(user_tda) = user_tda {
            (*to).gs = SegmentSelector::UserThreadDataArea as u32;
            (*to).fs = SegmentSelector::UserThreadDataArea as u32;
            Gdt::set_thread_data_area(user_tda.into());
        } else {
            (*to).gs = SegmentSelector::Null as u32;
            (*to).fs = SegmentSelector::Null as u32;
        }

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
        // Copy fields to local variables.
        let esp0: u32 = self.esp0;
        let cr3: u32 = self.cr3;
        let gs: u32 = self.gs;
        let fs: u32 = self.fs;
        let es: u32 = self.es;
        let ds: u32 = self.ds;
        let edi: u32 = self.edi;
        let esi: u32 = self.esi;
        let ebp: u32 = self.ebp;
        let edx: u32 = self.edx;
        let ecx: u32 = self.ecx;
        let ebx: u32 = self.ebx;
        let eax: u32 = self.eax;
        let err: u32 = self.err;
        let eip: u32 = self.eip;
        let cs: u32 = self.cs;
        let eflags: u32 = self.eflags;
        let esp: u32 = self.esp;
        let ss: u32 = self.ss;

        write!(
            f,
            "esp0={esp0:#010x}, cr3={cr3:#010x}, gs={gs:#010x}, fs={fs:#010x}, es={es:#010x}, \
             ds={ds:#010x}, edi={edi:#010x}, esi={esi:#010x}, ebp={ebp:#010x}, edx={edx:#010x}, \
             ecx={ecx:#010x}, ebx={ebx:#010x}, eax={eax:#010x}, err={err:#010x}, eip={eip:#010x}, \
             cs={cs:#010x}, eflags={eflags:#010x}, esp={esp:#010x}, ss={ss:#010x}",
        )
    }
}
