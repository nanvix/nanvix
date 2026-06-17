// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// C library types intentionally follow C naming conventions.
#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// The `sigjmp_buf` type stores the execution context for `sigsetjmp`/`siglongjmp`.
///
/// On x86-32, the following registers are saved: EBX, ESI, EDI, EBP, ESP, and the return address
/// (EIP). The `savemask` field records whether `sigsetjmp` was asked to save the signal mask.
///
#[repr(C)]
pub struct sigjmp_buf {
    /// Saved registers: EBX, ESI, EDI, EBP, ESP, EIP.
    pub regs: [c_int; 6],
    /// Nonzero if `sigsetjmp` was called with a nonzero `savemask` argument.
    pub savemask: c_int,
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::sigjmp_buf;
    use ::core::mem::{
        align_of,
        size_of,
    };
    use ::sysapi::ffi::c_int;

    // The sigsetjmp/siglongjmp assembly reuses the setjmp/longjmp register slots at byte offsets 0,
    // 4, 8, 12, 16, and 20, and stores the savemask flag at offset 24. These tests lock the
    // sigjmp_buf layout to that contract so the Rust type and the assembly cannot silently diverge.

    #[test]
    fn sigjmp_buf_has_six_register_slots() {
        let buf: sigjmp_buf = sigjmp_buf {
            regs: [0; 6],
            savemask: 0,
        };
        assert_eq!(buf.regs.len(), 6);
    }

    #[test]
    fn sigjmp_buf_size_matches_register_count_plus_savemask() {
        assert_eq!(size_of::<sigjmp_buf>(), 7 * size_of::<c_int>());
    }

    #[test]
    fn sigjmp_buf_alignment_matches_register() {
        assert_eq!(align_of::<sigjmp_buf>(), align_of::<c_int>());
    }

    #[test]
    fn sigjmp_buf_zero_initialized() {
        let buf: sigjmp_buf = sigjmp_buf {
            regs: [0; 6],
            savemask: 0,
        };
        assert!(buf.regs.iter().all(|&slot| slot == 0));
        assert_eq!(buf.savemask, 0);
    }
}
