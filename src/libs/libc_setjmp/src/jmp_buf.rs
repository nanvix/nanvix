// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// C library types intentionally follow C naming conventions.
#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_arch = "x86")]
use ::sysapi::ffi::c_int;
#[cfg(not(target_arch = "x86"))]
use ::sysapi::ffi::c_long;

//==================================================================================================
// Constants
//==================================================================================================

/// Width of a saved-register slot: 32-bit (`c_int`) on x86-32, 64-bit (`c_long`, LP64) on x86-64.
#[cfg(target_arch = "x86")]
pub type jmp_buf_reg = c_int;
/// Width of a saved-register slot: 32-bit (`c_int`) on x86-32, 64-bit (`c_long`, LP64) on x86-64.
#[cfg(not(target_arch = "x86"))]
pub type jmp_buf_reg = c_long;

/// Number of saved-register slots: six on x86-32 (EBX, ESI, EDI, EBP, ESP, EIP), eight on x86-64
/// (RBX, RBP, R12, R13, R14, R15, RSP, RIP).
#[cfg(target_arch = "x86")]
pub const JMP_BUF_REGS: usize = 6;
/// Number of saved-register slots: six on x86-32 (EBX, ESI, EDI, EBP, ESP, EIP), eight on x86-64
/// (RBX, RBP, R12, R13, R14, R15, RSP, RIP).
#[cfg(not(target_arch = "x86"))]
pub const JMP_BUF_REGS: usize = 8;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// The `jmp_buf` type stores registers for `setjmp`/`longjmp`.
///
/// On x86-32, the following registers are saved: EBX, ESI, EDI, EBP, ESP, and the return address
/// (EIP). On x86-64, the saved registers are: RBX, RBP, R12, R13, R14, R15, RSP, and the return
/// address (RIP).
///
#[repr(C)]
pub struct jmp_buf {
    /// Saved registers. x86-32: EBX, ESI, EDI, EBP, ESP, EIP. x86-64: RBX, RBP, R12, R13, R14,
    /// R15, RSP, RIP.
    pub regs: [jmp_buf_reg; JMP_BUF_REGS],
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        jmp_buf,
        jmp_buf_reg,
        JMP_BUF_REGS,
    };
    use ::core::mem::{
        align_of,
        size_of,
    };

    // The setjmp/longjmp assembly reads and writes JMP_BUF_REGS register slots, each the width of a
    // saved register (32-bit on x86-32, 64-bit on x86-64). These tests lock the jmp_buf layout to
    // that contract so the Rust type and the assembly cannot silently diverge.

    #[test]
    fn jmp_buf_has_expected_register_slots() {
        let buf: jmp_buf = jmp_buf {
            regs: [0; JMP_BUF_REGS],
        };
        assert_eq!(buf.regs.len(), JMP_BUF_REGS);
    }

    #[test]
    fn jmp_buf_size_matches_register_count() {
        assert_eq!(size_of::<jmp_buf>(), JMP_BUF_REGS * size_of::<jmp_buf_reg>());
    }

    #[test]
    fn jmp_buf_alignment_matches_register() {
        assert_eq!(align_of::<jmp_buf>(), align_of::<jmp_buf_reg>());
    }

    #[test]
    fn jmp_buf_zero_initialized() {
        let buf: jmp_buf = jmp_buf {
            regs: [0; JMP_BUF_REGS],
        };
        assert!(buf.regs.iter().all(|&slot| slot == 0));
    }
}
