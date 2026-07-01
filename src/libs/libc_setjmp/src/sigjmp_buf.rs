// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// C library types intentionally follow C naming conventions.
#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

use crate::jmp_buf::{
    jmp_buf_reg,
    JMP_BUF_REGS,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// The `sigjmp_buf` type stores the execution context for `sigsetjmp`/`siglongjmp`.
///
/// On x86-32, the following registers are saved: EBX, ESI, EDI, EBP, ESP, and the return address
/// (EIP). On x86-64, the saved registers are: RBX, RBP, R12, R13, R14, R15, RSP, and the return
/// address (RIP). The `savemask` field records whether `sigsetjmp` was asked to save the signal
/// mask.
///
#[repr(C)]
pub struct sigjmp_buf {
    /// Saved registers. x86-32: EBX, ESI, EDI, EBP, ESP, EIP. x86-64: RBX, RBP, R12, R13, R14,
    /// R15, RSP, RIP.
    pub regs: [jmp_buf_reg; JMP_BUF_REGS],
    /// Nonzero if `sigsetjmp` was called with a nonzero `savemask` argument.
    pub savemask: c_int,
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::sigjmp_buf;
    use crate::jmp_buf::{
        jmp_buf_reg,
        JMP_BUF_REGS,
    };
    use ::core::mem::{
        align_of,
        offset_of,
        size_of,
    };

    // The sigsetjmp/siglongjmp assembly reuses the setjmp/longjmp register slots and stores the
    // savemask flag immediately after them. These tests lock the sigjmp_buf layout to that contract
    // so the Rust type and the assembly cannot silently diverge.

    #[test]
    fn sigjmp_buf_has_expected_register_slots() {
        let buf: sigjmp_buf = sigjmp_buf {
            regs: [0; JMP_BUF_REGS],
            savemask: 0,
        };
        assert_eq!(buf.regs.len(), JMP_BUF_REGS);
    }

    #[test]
    fn sigjmp_buf_savemask_follows_registers() {
        // The sigsetjmp assembly stores the savemask flag at the byte offset immediately past the
        // register block.
        assert_eq!(offset_of!(sigjmp_buf, savemask), JMP_BUF_REGS * size_of::<jmp_buf_reg>());
    }

    #[test]
    fn sigjmp_buf_alignment_matches_register() {
        assert_eq!(align_of::<sigjmp_buf>(), align_of::<jmp_buf_reg>());
    }

    #[test]
    fn sigjmp_buf_zero_initialized() {
        let buf: sigjmp_buf = sigjmp_buf {
            regs: [0; JMP_BUF_REGS],
            savemask: 0,
        };
        assert!(buf.regs.iter().all(|&slot| slot == 0));
        assert_eq!(buf.savemask, 0);
    }
}
