// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// C library types intentionally follow C naming conventions.
#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::c_int,
    signal::sigset_t,
};

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
/// address (RIP). On AArch64, X19-X30, SP, and D8-D15 are saved. The remaining fields record
/// whether `sigsetjmp` saved the signal mask and, when requested, the saved mask itself.
///
#[repr(C)]
pub struct sigjmp_buf {
    /// Saved registers for the active architecture.
    pub regs: [jmp_buf_reg; JMP_BUF_REGS],
    /// Nonzero if `sigsetjmp` was called with a nonzero `savemask` argument.
    pub savemask: c_int,
    /// Signal mask saved by `sigsetjmp` when `savemask` is nonzero.
    pub sigmask: sigset_t,
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
    use ::sysapi::signal::sigset_t;

    // The sigsetjmp/siglongjmp assembly reuses the setjmp/longjmp register slots. These tests lock
    // the sigjmp_buf layout to the matching C declaration so the Rust type and generated header
    // cannot silently diverge.

    #[test]
    fn sigjmp_buf_has_expected_register_slots() {
        let buf: sigjmp_buf = sigjmp_buf {
            regs: [0; JMP_BUF_REGS],
            savemask: 0,
            sigmask: 0,
        };
        assert_eq!(buf.regs.len(), JMP_BUF_REGS);
    }

    #[test]
    fn sigjmp_buf_savemask_follows_registers() {
        // Keep the pre-existing savemask offset immediately after the register block.
        assert_eq!(offset_of!(sigjmp_buf, savemask), JMP_BUF_REGS * size_of::<jmp_buf_reg>());
    }

    #[test]
    fn sigjmp_buf_sigmask_follows_savemask_with_c_alignment() {
        let savemask_end: usize =
            offset_of!(sigjmp_buf, savemask) + size_of::<::sysapi::ffi::c_int>();
        let sigmask_alignment: usize = align_of::<sigset_t>();
        let expected_offset: usize = savemask_end.next_multiple_of(sigmask_alignment);
        assert_eq!(offset_of!(sigjmp_buf, sigmask), expected_offset);
    }

    #[test]
    fn sigjmp_buf_alignment_matches_largest_field() {
        assert_eq!(align_of::<sigjmp_buf>(), align_of::<jmp_buf_reg>().max(align_of::<sigset_t>()));
    }

    #[test]
    fn sigjmp_buf_zero_initialized() {
        let buf: sigjmp_buf = sigjmp_buf {
            regs: [0; JMP_BUF_REGS],
            savemask: 0,
            sigmask: 0,
        };
        assert!(buf.regs.iter().all(|&slot| slot == 0));
        assert_eq!(buf.savemask, 0);
        assert_eq!(buf.sigmask, 0);
    }
}
