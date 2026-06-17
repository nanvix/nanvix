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
/// The `jmp_buf` type stores registers for `setjmp`/`longjmp`.
///
/// On x86-32, the following registers are saved: EBX, ESI, EDI, EBP, ESP, and the return address
/// (EIP).
///
#[repr(C)]
pub struct jmp_buf {
    /// Saved registers: EBX, ESI, EDI, EBP, ESP, EIP.
    pub regs: [c_int; 6],
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::jmp_buf;
    use ::core::mem::{
        align_of,
        size_of,
    };
    use ::sysapi::ffi::c_int;

    // The setjmp/longjmp assembly reads and writes six register slots at byte offsets 0, 4, 8, 12,
    // 16, and 20. These tests lock the jmp_buf layout to that contract so the Rust type and the
    // assembly cannot silently diverge.

    #[test]
    fn jmp_buf_has_six_register_slots() {
        let buf: jmp_buf = jmp_buf { regs: [0; 6] };
        assert_eq!(buf.regs.len(), 6);
    }

    #[test]
    fn jmp_buf_size_matches_register_count() {
        assert_eq!(size_of::<jmp_buf>(), 6 * size_of::<c_int>());
    }

    #[test]
    fn jmp_buf_alignment_matches_register() {
        assert_eq!(align_of::<jmp_buf>(), align_of::<c_int>());
    }

    #[test]
    fn jmp_buf_zero_initialized() {
        let buf: jmp_buf = jmp_buf { regs: [0; 6] };
        assert!(buf.regs.iter().all(|&slot| slot == 0));
    }
}
