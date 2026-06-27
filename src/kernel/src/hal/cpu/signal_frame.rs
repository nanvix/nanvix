// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::arch::SignalCpuContext;

//==================================================================================================
// Constants
//==================================================================================================

/// Magic value stamped into a signal frame so that `sigreturn()` can reject a frame that was not
/// produced by the kernel (a corrupted stack or a forged frame).
pub const SIGFRAME_MAGIC: u32 = 0x5347_4652;

/// Size, in bytes, of the architecture FPU save area (`FXSAVE`/`FXRSTOR` region).
pub const FPU_AREA_SIZE: usize = 512;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// The save area of a signal frame, written to and read back from user space verbatim.
///
/// Instances are copied across the kernel-call boundary as raw bytes, so the layout is fixed
/// (`#[repr(C)]`) and every field is a plain integer or byte array. The FPU area is a raw
/// `FXSAVE`/`FXRSTOR` buffer; the kernel performs the FPU save and restore through a properly
/// aligned scratch buffer, so this field needs no special alignment of its own.
///
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SigFrame {
    /// Validation magic; see [`SIGFRAME_MAGIC`].
    pub magic: u32,
    /// Non-zero when the frame carries `SA_SIGINFO` arguments.
    pub has_siginfo: u32,
    /// Blocked-signal mask to restore when the handler returns.
    pub blocked: u64,
    /// Minimal `siginfo_t` image referenced by an `SA_SIGINFO` handler's second argument.
    ///
    /// Only the leading fields (`si_signo`, `si_errno`, `si_code`) are populated; the remainder is
    /// zero. This is enough for a handler to read the delivered signal number without faulting.
    pub siginfo: [u32; 8],
    /// Saved CPU context of the interrupted thread.
    pub cpu: SignalCpuContext,
    /// Saved FPU state (`FXSAVE` image) of the interrupted thread.
    pub fpu: [u8; FPU_AREA_SIZE],
}

///
/// # Description
///
/// Computed placement of a signal frame on a target thread's user stack.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameLayout {
    /// Lowest address of the frame and the stack pointer the handler is entered with. The handler's
    /// return address lives here.
    pub frame_top: usize,
    /// Address at which the [`SigFrame`] save area begins.
    pub save_area_base: usize,
}

///
/// # Description
///
/// Reasons why a frame presented to `sigreturn()` is rejected.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SigFrameError {
    /// The frame's magic value did not match [`SIGFRAME_MAGIC`].
    BadMagic,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the largest address less than or equal to `value` that is congruent to `residue` modulo
/// `align`.
///
/// # Parameters
///
/// - `value`: The upper bound to round down from.
/// - `align`: The alignment modulus (a power of two).
/// - `residue`: The required residue modulo `align`.
///
/// # Returns
///
/// The rounded-down address.
///
pub fn align_down_residue(value: usize, align: usize, residue: usize) -> usize {
    let r: usize = value % align;
    if r >= residue {
        value - (r - residue)
    } else {
        value - (r + align - residue)
    }
}

///
/// # Description
///
/// Builds the save area of a signal frame.
///
/// # Parameters
///
/// - `cpu`: Snapshot of the interrupted CPU context.
/// - `blocked`: The blocked mask to restore on `sigreturn()` (the mask in effect before delivery).
/// - `fpu`: The saved FPU image of the interrupted thread.
/// - `signum`: The signal being delivered (1-based), recorded in the embedded `siginfo`.
/// - `has_siginfo`: Whether the frame carries `SA_SIGINFO` arguments.
///
/// # Returns
///
/// The populated [`SigFrame`].
///
pub fn build_frame(
    cpu: SignalCpuContext,
    blocked: u64,
    fpu: [u8; FPU_AREA_SIZE],
    signum: usize,
    has_siginfo: bool,
) -> SigFrame {
    let mut siginfo: [u32; 8] = [0u32; 8];
    // si_signo is the first member of the minimal siginfo image.
    siginfo[0] = signum as u32;
    SigFrame {
        magic: SIGFRAME_MAGIC,
        has_siginfo: u32::from(has_siginfo),
        blocked,
        siginfo,
        cpu,
        fpu,
    }
}

///
/// # Description
///
/// Returns the byte offset of the saved CPU context within a [`SigFrame`].
///
/// Used as the third (`*ctx`) handler argument for an `SA_SIGINFO` handler.
///
pub fn ctx_offset() -> usize {
    core::mem::offset_of!(SigFrame, cpu)
}

///
/// # Description
///
/// Returns the byte offset of the embedded `siginfo` image within a [`SigFrame`].
///
/// Used as the second (`*info`) handler argument for an `SA_SIGINFO` handler.
///
pub fn siginfo_offset() -> usize {
    core::mem::offset_of!(SigFrame, siginfo)
}
