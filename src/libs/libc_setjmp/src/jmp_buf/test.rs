// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
