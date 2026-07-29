// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::x86::cpu::tss;
use ::arch::cpu::{
    ring::PrivilegeLevel,
    tss::Tss,
};
use ::core::arch::asm;
use ::sys::mm::{
    Address,
    VirtualAddress,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Flags installed when a thread enters a signal handler: interrupts enabled (bit 9) and the
/// always-one reserved bit (bit 1), everything else clear. Mirrors the kernel-call delivery path.
const SIGNAL_HANDLER_ENTRY_FLAGS: u64 = (1 << 9) | (1 << 1);

/// Mask for the requested-privilege-level (RPL) field of a segment selector (its low two bits).
const SELECTOR_RPL_MASK: u64 = 0b11;

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

///
/// # Description
///
/// Snapshot of the interrupted user CPU context saved into a signal frame and restored by
/// `sigreturn()` (x86-64 variant).
///
/// The field set mirrors the general-purpose register file of the target architecture plus the
/// instruction pointer, stack pointer, flags, and the code and stack segment selectors. The
/// selectors and flags are sanitized on restore so a forged frame cannot escalate privilege.
///
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct SignalCpuContext {
    /// Instruction pointer (`RIP`).
    pub ip: u64,
    /// Stack pointer (`RSP`).
    pub sp: u64,
    /// Flags register (`RFLAGS`).
    pub flags: u64,
    /// `RAX` (also the interrupted kernel call's return value).
    pub ax: u64,
    /// `RBX`.
    pub bx: u64,
    /// `RCX`.
    pub cx: u64,
    /// `RDX`.
    pub dx: u64,
    /// `RSI`.
    pub si: u64,
    /// `RDI`.
    pub di: u64,
    /// `RBP`.
    pub bp: u64,
    /// `R8`.
    pub r8: u64,
    /// `R9`.
    pub r9: u64,
    /// `R10`.
    pub r10: u64,
    /// `R11`.
    pub r11: u64,
    /// `R12`.
    pub r12: u64,
    /// `R13`.
    pub r13: u64,
    /// `R14`.
    pub r14: u64,
    /// `R15`.
    pub r15: u64,
    /// Code segment selector (`CS`).
    pub cs: u64,
    /// Stack segment selector (`SS`).
    pub ss: u64,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ContextInformation {
    /// Byte offset of the `rsp0` field within the structure.
    pub const CONTEXT_RSP0: u32 = core::mem::offset_of!(Self, rsp0) as u32;
    /// Byte offset of the `cr3` field within the structure.
    pub const CONTEXT_CR3: u32 = core::mem::offset_of!(Self, cr3) as u32;
    /// Byte offset of the `r15` field within the structure.
    pub const CONTEXT_R15: u32 = core::mem::offset_of!(Self, r15) as u32;
    /// Byte offset of the `r14` field within the structure.
    pub const CONTEXT_R14: u32 = core::mem::offset_of!(Self, r14) as u32;
    /// Byte offset of the `r13` field within the structure.
    pub const CONTEXT_R13: u32 = core::mem::offset_of!(Self, r13) as u32;
    /// Byte offset of the `r12` field within the structure.
    pub const CONTEXT_R12: u32 = core::mem::offset_of!(Self, r12) as u32;
    /// Byte offset of the `r11` field within the structure.
    pub const CONTEXT_R11: u32 = core::mem::offset_of!(Self, r11) as u32;
    /// Byte offset of the `r10` field within the structure.
    pub const CONTEXT_R10: u32 = core::mem::offset_of!(Self, r10) as u32;
    /// Byte offset of the `r9` field within the structure.
    pub const CONTEXT_R9: u32 = core::mem::offset_of!(Self, r9) as u32;
    /// Byte offset of the `r8` field within the structure.
    pub const CONTEXT_R8: u32 = core::mem::offset_of!(Self, r8) as u32;
    /// Byte offset of the `rbp` field within the structure.
    pub const CONTEXT_RBP: u32 = core::mem::offset_of!(Self, rbp) as u32;
    /// Byte offset of the `rsi` field within the structure.
    pub const CONTEXT_RSI: u32 = core::mem::offset_of!(Self, rsi) as u32;
    /// Byte offset of the `rdi` field within the structure.
    pub const CONTEXT_RDI: u32 = core::mem::offset_of!(Self, rdi) as u32;
    /// Byte offset of the `rdx` field within the structure.
    pub const CONTEXT_RDX: u32 = core::mem::offset_of!(Self, rdx) as u32;
    /// Byte offset of the `rcx` field within the structure.
    pub const CONTEXT_RCX: u32 = core::mem::offset_of!(Self, rcx) as u32;
    /// Byte offset of the `rbx` field within the structure.
    pub const CONTEXT_RBX: u32 = core::mem::offset_of!(Self, rbx) as u32;
    /// Byte offset of the `rax` field within the structure.
    pub const CONTEXT_RAX: u32 = core::mem::offset_of!(Self, rax) as u32;
    /// Byte offset of the `err` field within the structure.
    pub const CONTEXT_ERR: u32 = core::mem::offset_of!(Self, err) as u32;
    /// Byte offset of the `rip` field within the structure.
    pub const CONTEXT_RIP: u32 = core::mem::offset_of!(Self, rip) as u32;
    /// Byte offset of the `cs` field within the structure.
    #[allow(dead_code)]
    pub const CONTEXT_CS: u32 = core::mem::offset_of!(Self, cs) as u32;
    /// Byte offset of the `rflags` field within the structure.
    pub const CONTEXT_RFLAGS: u32 = core::mem::offset_of!(Self, rflags) as u32;
    /// Byte offset of the `rsp` field within the structure.
    pub const CONTEXT_RSP: u32 = core::mem::offset_of!(Self, rsp) as u32;
    /// Byte offset of the `ss` field within the structure.
    #[allow(dead_code)]
    pub const CONTEXT_SS: u32 = core::mem::offset_of!(Self, ss) as u32;

    /// Size of the software-saved portion of the context (bytes before `err`).
    pub const CONTEXT_SW_SIZE: u32 = Self::CONTEXT_ERR;

    /// Size of the hardware-saved portion of the context (bytes from `err` onward).
    #[allow(dead_code)]
    pub const CONTEXT_HW_SIZE: u32 = core::mem::size_of::<Self>() as u32 - Self::CONTEXT_ERR;

    #[allow(clippy::as_conversions)]
    pub fn new(cr3: usize, rsp: usize, rsp0: usize) -> Self {
        Self {
            rsp0: rsp0 as u64,
            cr3: cr3 as u64,
            rsp: rsp as u64,
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
            ip: self.rip,
            sp: self.rsp,
            flags: self.rflags,
            ax: self.rax,
            bx: self.rbx,
            cx: self.rcx,
            dx: self.rdx,
            si: self.rsi,
            di: self.rdi,
            bp: self.rbp,
            r8: self.r8,
            r9: self.r9,
            r10: self.r10,
            r11: self.r11,
            r12: self.r12,
            r13: self.r13,
            r14: self.r14,
            r15: self.r15,
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
    /// The instruction pointer is pointed at the handler entry, the stack pointer at the top of the
    /// signal frame, and the flags reset to a clean handler-entry value (interrupts enabled,
    /// everything else clear). The signal number is placed in `RDI`, the first integer-argument
    /// register of the System V ABI, so a handler declared as `fn(int)` receives it. For an
    /// `SA_SIGINFO` handler the user addresses of the embedded `siginfo` and context images are
    /// placed in `RSI` and `RDX`, the second and third integer-argument registers, so a handler
    /// declared as `fn(int, *const siginfo_t, *const c_void)` receives them; a non-`SA_SIGINFO`
    /// handler passes `0` for both and a one-argument handler ignores them.
    ///
    /// # Parameters
    ///
    /// - `entry`: Address of the user-space signal handler.
    /// - `frame_top`: Stack pointer the handler is entered with (top of the signal frame).
    /// - `signum`: The signal number delivered to the handler.
    /// - `info_ptr`: User address of the embedded `siginfo` image (`SA_SIGINFO`), or `0`.
    /// - `ctx_ptr`: User address of the embedded context image (`SA_SIGINFO`), or `0`.
    ///
    pub fn redirect_to_signal_handler(
        &mut self,
        entry: usize,
        frame_top: usize,
        signum: usize,
        info_ptr: usize,
        ctx_ptr: usize,
    ) {
        self.rip = entry as u64;
        self.rsp = frame_top as u64;
        self.rflags = SIGNAL_HANDLER_ENTRY_FLAGS;
        self.rdi = signum as u64;
        self.rsi = info_ptr as u64;
        self.rdx = ctx_ptr as u64;
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
        let cs: u64 = self.cs;
        (cs & SELECTOR_RPL_MASK) == PrivilegeLevel::Ring3 as u64
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
