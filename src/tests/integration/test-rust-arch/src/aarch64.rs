// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use crate::test;
use ::core::arch::asm;

fn test_integer_addition() -> bool {
    const LEFT: u64 = 0x1234_5678_9abc_def0;
    const RIGHT: u64 = 0x1111_2222_3333_4444;
    const EXPECTED: u64 = LEFT.wrapping_add(RIGHT);

    let result: u64;
    unsafe {
        asm!(
            "add {result}, {left}, {right}",
            result = out(reg) result,
            left = in(reg) LEFT,
            right = in(reg) RIGHT,
            options(nomem, nostack, preserves_flags)
        );
    }

    result == EXPECTED
}

fn test_yield() -> bool {
    ::arch::cpu::pause();
    true
}

pub fn test_aarch64() -> bool {
    let mut all_passed: bool = true;

    all_passed &= test!(test_integer_addition());
    all_passed &= test!(test_yield());

    all_passed
}
