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
use ::arch::cpu::{
    ring::PrivilegeLevel,
    tss::Tss,
};
use ::sys::mm::VirtualAddress;

//==================================================================================================
// Constants
//==================================================================================================

/// Flags installed when a thread enters a signal handler: interrupts enabled (bit 9) and the
/// always-one reserved bit (bit 1), everything else clear. Mirrors the kernel-call delivery path.
const SIGNAL_HANDLER_ENTRY_FLAGS: u32 = (1 << 9) | (1 << 1);

/// Mask for the requested-privilege-level (RPL) field of a segment selector (its low two bits).
const SELECTOR_RPL_MASK: u32 = 0b11;

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

// `ContextInformation` must be 76 bytes long. This must match low-level assembly dispatcher code.
::static_assert::assert_eq_size!(ContextInformation, 76);

///
/// # Description
///
/// Snapshot of the interrupted user CPU context saved into a signal frame and restored by
/// `sigreturn()`.
///
/// The field set mirrors the general-purpose register file of the target architecture plus the
/// instruction pointer, stack pointer, flags, and the code and stack segment selectors. The
/// selectors and flags are sanitized on restore so a forged frame cannot escalate privilege.
///
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct SignalCpuContext {
    /// Instruction pointer (`EIP`).
    pub ip: u32,
    /// Stack pointer (`ESP`).
    pub sp: u32,
    /// Flags register (`EFLAGS`).
    pub flags: u32,
    /// `EAX` (also the interrupted kernel call's return value).
    pub ax: u32,
    /// `EBX`.
    pub bx: u32,
    /// `ECX`.
    pub cx: u32,
    /// `EDX`.
    pub dx: u32,
    /// `ESI`.
    pub si: u32,
    /// `EDI`.
    pub di: u32,
    /// `EBP`.
    pub bp: u32,
    /// Code segment selector (`CS`).
    pub cs: u32,
    /// Stack segment selector (`SS`).
    pub ss: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ContextInformation {
    /// Byte offset of the `esp0` field within the structure.
    pub const CONTEXT_ESP0: u32 = core::mem::offset_of!(Self, esp0) as u32;
    /// Byte offset of the `cr3` field within the structure.
    pub const CONTEXT_CR3: u32 = core::mem::offset_of!(Self, cr3) as u32;
    /// Byte offset of the `gs` field within the structure.
    pub const CONTEXT_GS: u32 = core::mem::offset_of!(Self, gs) as u32;
    /// Byte offset of the `fs` field within the structure.
    pub const CONTEXT_FS: u32 = core::mem::offset_of!(Self, fs) as u32;
    /// Byte offset of the `es` field within the structure.
    pub const CONTEXT_ES: u32 = core::mem::offset_of!(Self, es) as u32;
    /// Byte offset of the `ds` field within the structure.
    pub const CONTEXT_DS: u32 = core::mem::offset_of!(Self, ds) as u32;
    /// Byte offset of the `edi` field within the structure.
    pub const CONTEXT_EDI: u32 = core::mem::offset_of!(Self, edi) as u32;
    /// Byte offset of the `esi` field within the structure.
    pub const CONTEXT_ESI: u32 = core::mem::offset_of!(Self, esi) as u32;
    /// Byte offset of the `ebp` field within the structure.
    pub const CONTEXT_EBP: u32 = core::mem::offset_of!(Self, ebp) as u32;
    /// Byte offset of the `edx` field within the structure.
    pub const CONTEXT_EDX: u32 = core::mem::offset_of!(Self, edx) as u32;
    /// Byte offset of the `ecx` field within the structure.
    pub const CONTEXT_ECX: u32 = core::mem::offset_of!(Self, ecx) as u32;
    /// Byte offset of the `ebx` field within the structure.
    pub const CONTEXT_EBX: u32 = core::mem::offset_of!(Self, ebx) as u32;
    /// Byte offset of the `eax` field within the structure.
    pub const CONTEXT_EAX: u32 = core::mem::offset_of!(Self, eax) as u32;
    /// Byte offset of the `err` field within the structure.
    pub const CONTEXT_ERR: u32 = core::mem::offset_of!(Self, err) as u32;
    /// Byte offset of the `eip` field within the structure.
    pub const CONTEXT_EIP: u32 = core::mem::offset_of!(Self, eip) as u32;
    /// Byte offset of the `eflags` field within the structure.
    pub const CONTEXT_EFLAGS: u32 = core::mem::offset_of!(Self, eflags) as u32;
    /// Byte offset of the `esp` field within the structure.
    pub const CONTEXT_ESP: u32 = core::mem::offset_of!(Self, esp) as u32;

    /// Size of the software-saved portion of the context (bytes before `err`).
    pub const CONTEXT_SW_SIZE: u32 = Self::CONTEXT_ERR;

    /// Size of the hardware-saved portion of the context (bytes from `err` onward).
    #[cfg(feature = "exception-stack-guard")]
    pub const CONTEXT_HW_SIZE: u32 = core::mem::size_of::<Self>() as u32 - Self::CONTEXT_ERR;

    #[allow(clippy::as_conversions)]
    pub fn new(cr3: usize, esp: usize, esp0: usize) -> Self {
        Self {
            esp0: esp0 as u32,
            cr3: cr3 as u32,
            esp: esp as u32,
            ..Default::default()
        }
    }

    ///
    /// # Description
    ///
    /// Reads the interrupted user context saved by an exception into the architecture-neutral
    /// [`SignalCpuContext`] used to build a signal frame.
    ///
    /// # Returns
    ///
    /// The [`SignalCpuContext`] mirroring this saved exception context.
    ///
    pub fn to_signal_context(&self) -> SignalCpuContext {
        // Copy the packed fields by value (a reference into a packed struct is ill-formed).
        SignalCpuContext {
            ip: self.eip,
            sp: self.esp,
            flags: self.eflags,
            ax: self.eax,
            bx: self.ebx,
            cx: self.ecx,
            dx: self.edx,
            si: self.esi,
            di: self.edi,
            bp: self.ebp,
            cs: self.cs,
            ss: self.ss,
        }
    }

    ///
    /// # Description
    ///
    /// Rewrites this saved exception context so that, on return to user mode, the faulting thread
    /// enters a signal handler on its freshly built signal frame.
    ///
    /// Mirrors [`redirect_to_handler`](crate::hal::arch::redirect_to_handler) for the kernel-call
    /// path: the instruction pointer is pointed at the handler entry, the stack pointer at the top
    /// of the signal frame, and the flags reset to a clean handler-entry value (interrupts enabled,
    /// everything else clear).
    ///
    /// # Parameters
    ///
    /// - `entry`: Address of the user-space signal handler.
    /// - `frame_top`: Stack pointer the handler is entered with (top of the signal frame).
    /// - `_signum`: The signal number delivered to the handler. Unused on x86, where the cdecl ABI
    ///   passes it on the stack (written into the frame by the frame builder).
    /// - `_info_ptr`/`_ctx_ptr`: The `SA_SIGINFO` siginfo/context pointers. Unused on x86 for the
    ///   same reason as `_signum`.
    ///
    pub fn redirect_to_signal_handler(
        &mut self,
        entry: usize,
        frame_top: usize,
        _signum: usize,
        _info_ptr: usize,
        _ctx_ptr: usize,
    ) {
        self.eip = entry as u32;
        self.esp = frame_top as u32;
        self.eflags = SIGNAL_HANDLER_ENTRY_FLAGS;
    }

    ///
    /// # Description
    ///
    /// Returns whether this saved exception context resumes in user mode, determined from the
    /// requested-privilege-level of the saved code-segment selector.
    ///
    /// # Returns
    ///
    /// `true` if the interrupted context is ring 3, `false` otherwise.
    ///
    pub fn returns_to_user(&self) -> bool {
        let cs: u32 = self.cs;
        (cs & SELECTOR_RPL_MASK) == PrivilegeLevel::Ring3 as u32
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
        // Update the GDT TDA entry base and write the appropriate selector
        // into `(*to).gs` / `(*to).fs`.  `__context_switch()` restores
        // `%gs`/`%fs` from these fields, which forces the CPU to re-read the
        // GDT entry and refresh the hidden descriptor cache.
        if let Some(user_tda) = user_tda {
            (*to).gs = SegmentSelector::UserThreadDataArea as u32;
            (*to).fs = SegmentSelector::UserThreadDataArea as u32;
            Gdt::set_thread_data_area_base(user_tda.into());
        } else {
            // Clear %gs/%fs so they do not reference a stale TDA.
            // Also zero the GDT entry base so a stale selector load cannot
            // silently reference the old TDA address.
            (*to).gs = SegmentSelector::Null as u32;
            (*to).fs = SegmentSelector::Null as u32;
            Gdt::set_thread_data_area_base(0);
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
