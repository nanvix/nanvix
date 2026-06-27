// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    align_down_residue,
    build_frame,
    frame_layout,
    next_blocked,
    save_area_offset_from_sigreturn_sp,
    validate_and_restore,
    SigFrame,
    SigFrameError,
    SignalCpuContext,
    RETADDR_SIZE,
    SIGFRAME_MAGIC,
};
use crate::hal::arch::x86::mem::gdt::SegmentSelector;

//==================================================================================================
// Helpers
//==================================================================================================

/// Bit for signal `signum` (1-based) in a signal set.
fn bit(signum: usize) -> u64 {
    1u64 << (signum - 1)
}

/// Builds a CPU context with distinctive, easily recognizable register values.
fn sample_cpu() -> SignalCpuContext {
    SignalCpuContext {
        ip: 0x0040_1000,
        sp: 0x7fff_e000,
        flags: 0x0000_0202,
        ax: 0x1111_1111,
        bx: 0x2222_2222,
        dx: 0x3333_3333,
        ..Default::default()
    }
}

/// Builds a zeroed FPU image with a recognizable byte pattern.
fn sample_fpu() -> [u8; 512] {
    let mut fpu: [u8; 512] = [0u8; 512];
    fpu[0] = 0xAB;
    fpu[511] = 0xCD;
    fpu
}

//==================================================================================================
// Alignment Tests
//==================================================================================================

///
/// # Description
///
/// `align_down_residue` rounds down to the requested residue without ever rounding up.
///
fn test_align_down_residue() -> bool {
    // Already at the residue: unchanged.
    if align_down_residue(0x1000 + 12, 16, 12) != 0x1000 + 12 {
        error!("align_down_residue changed an already-aligned value");
        return false;
    }
    // Above the residue within a block: rounds down to the residue.
    if align_down_residue(0x1000 + 15, 16, 12) != 0x1000 + 12 {
        error!("align_down_residue did not round down to the residue");
        return false;
    }
    // Below the residue within a block: rounds down to the previous block's residue.
    if align_down_residue(0x1000 + 4, 16, 12) != 0x1000 + 12 - 16 {
        error!("align_down_residue did not cross to the previous block");
        return false;
    }
    true
}

///
/// # Description
///
/// `frame_layout` places the frame below the stack pointer, leaves room for the whole frame, and
/// aligns the handler entry so the ABI stack-alignment invariant holds.
///
fn test_frame_layout_alignment() -> bool {
    let user_sp: usize = 0x7fff_f000;
    let layout = match frame_layout(user_sp) {
        Some(layout) => layout,
        None => {
            error!("frame_layout returned None for a valid stack pointer");
            return false;
        },
    };

    // The frame must lie entirely below the stack pointer.
    if layout.frame_top >= user_sp {
        error!("frame_top is not below the user stack pointer");
        return false;
    }
    let total: usize =
        RETADDR_SIZE + save_area_offset_from_sigreturn_sp() + core::mem::size_of::<SigFrame>();
    if user_sp - layout.frame_top < total {
        error!("frame_layout did not reserve room for the whole frame");
        return false;
    }

    // At handler entry the stack pointer is `frame_top`; the ABI requires `sp + retaddr` to be
    // 16-byte aligned.
    if (layout.frame_top + RETADDR_SIZE) % 16 != 0 {
        error!("handler entry stack pointer is misaligned");
        return false;
    }

    // The save area sits just past the return address and the on-stack arguments.
    if layout.save_area_base
        != layout.frame_top + RETADDR_SIZE + save_area_offset_from_sigreturn_sp()
    {
        error!("save area base is not where it is expected");
        return false;
    }
    true
}

///
/// # Description
///
/// `frame_layout` reports failure when the stack pointer is too low to hold a frame.
///
fn test_frame_layout_underflow() -> bool {
    if frame_layout(8).is_some() {
        error!("frame_layout accepted an impossibly low stack pointer");
        return false;
    }
    true
}

//==================================================================================================
// Mask Tests
//==================================================================================================

///
/// # Description
///
/// Without `SA_NODEFER`, the delivered signal is added to the in-handler blocked mask alongside the
/// handler's own mask.
///
fn test_next_blocked_defers_signal() -> bool {
    let next: u64 = next_blocked(bit(2), bit(10), 4, false);
    if next != bit(2) | bit(10) | bit(4) {
        error!("next_blocked did not block the delivered signal");
        return false;
    }
    true
}

///
/// # Description
///
/// With `SA_NODEFER`, the delivered signal is not added to the in-handler blocked mask.
///
fn test_next_blocked_nodefer() -> bool {
    let next: u64 = next_blocked(bit(2), bit(10), 4, true);
    if next != bit(2) | bit(10) {
        error!("next_blocked blocked the delivered signal despite SA_NODEFER");
        return false;
    }
    true
}

//==================================================================================================
// Round-Trip and Validation Tests
//==================================================================================================

///
/// # Description
///
/// A frame built by the kernel round-trips through validation: the general-purpose registers, the
/// blocked mask, and the FPU image are preserved.
///
fn test_build_restore_round_trip() -> bool {
    let cpu: SignalCpuContext = sample_cpu();
    let blocked: u64 = bit(3) | bit(15);
    let fpu: [u8; 512] = sample_fpu();

    let frame: SigFrame = build_frame(cpu, blocked, fpu, 11, false);

    if frame.magic != SIGFRAME_MAGIC {
        error!("built frame is missing its magic");
        return false;
    }
    if frame.blocked != blocked {
        error!("built frame did not preserve the blocked mask");
        return false;
    }
    if frame.fpu[0] != 0xAB || frame.fpu[511] != 0xCD {
        error!("built frame did not preserve the FPU image");
        return false;
    }

    let restored: SignalCpuContext = match validate_and_restore(&frame) {
        Ok(restored) => restored,
        Err(_) => {
            error!("validation rejected a well-formed frame");
            return false;
        },
    };

    // The general-purpose registers, instruction pointer, and stack pointer round-trip unchanged.
    if restored.ip != cpu.ip
        || restored.sp != cpu.sp
        || restored.ax != cpu.ax
        || restored.bx != cpu.bx
        || restored.dx != cpu.dx
    {
        error!("round-trip altered the general-purpose registers");
        return false;
    }
    true
}

///
/// # Description
///
/// A frame whose magic is wrong is rejected.
///
fn test_validate_rejects_bad_magic() -> bool {
    let mut frame: SigFrame = build_frame(sample_cpu(), 0, sample_fpu(), 11, false);
    frame.magic ^= 0xFFFF_FFFF;
    match validate_and_restore(&frame) {
        Err(SigFrameError::BadMagic) => true,
        _ => {
            error!("validation accepted a frame with a corrupt magic");
            false
        },
    }
}

///
/// # Description
///
/// Validation forces the segment selectors to the user selectors and reduces the flags to safe
/// user values, even when the frame claims privileged ones.
///
fn test_validate_sanitizes_privileged_state() -> bool {
    let mut cpu: SignalCpuContext = sample_cpu();
    // Claim a kernel-like code selector, a bogus stack selector, the trap flag, and a raised I/O
    // privilege level.
    cpu.cs = 0x08;
    cpu.ss = 0x10;
    cpu.flags = (1 << 8) | (3 << 12); // TF | IOPL=3
    let frame: SigFrame = build_frame(cpu, 0, sample_fpu(), 11, false);

    let restored: SignalCpuContext = match validate_and_restore(&frame) {
        Ok(restored) => restored,
        Err(_) => {
            error!("validation rejected a frame it should have sanitized");
            return false;
        },
    };

    if restored.cs != SegmentSelector::UserCode as _ {
        error!("validation did not force the user code selector");
        return false;
    }
    if restored.ss != SegmentSelector::UserData as _ {
        error!("validation did not force the user data selector");
        return false;
    }
    // Interrupts must be enabled, the trap flag cleared, and the I/O privilege level zeroed.
    if restored.flags & (1 << 9) == 0 {
        error!("validation did not force interrupts enabled");
        return false;
    }
    if restored.flags & (1 << 8) != 0 {
        error!("validation did not clear the trap flag");
        return false;
    }
    if restored.flags & (3 << 12) != 0 {
        error!("validation did not clear the I/O privilege level");
        return false;
    }
    true
}

//==================================================================================================
// Test Runner
//==================================================================================================

///
/// # Description
///
/// Runs all signal-frame unit tests.
///
/// # Returns
///
/// `true` if every test passed, `false` otherwise.
///
pub fn test() -> bool {
    let mut passed: bool = true;
    passed &= test_align_down_residue();
    passed &= test_frame_layout_alignment();
    passed &= test_frame_layout_underflow();
    passed &= test_next_blocked_defers_signal();
    passed &= test_next_blocked_nodefer();
    passed &= test_build_restore_round_trip();
    passed &= test_validate_rejects_bad_magic();
    passed &= test_validate_sanitizes_privileged_state();
    passed
}
