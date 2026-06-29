// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Issues a kernel call with no arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall0(kcall_nr: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with one argument.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall1(kcall_nr: u32, arg0: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with two arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall2(kcall_nr: u32, arg0: u32, arg1: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            in("rsi") arg1,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with three arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall3(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            options(nostack, preserves_flags)
        );
    }
    ret
}

/// Issues a kernel call with four arguments.
///
/// # Safety
///
/// This function is unsafe because it issues inline assembly.
#[inline(never)]
pub unsafe fn kcall4(kcall_nr: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i64 {
    let ret: i64;
    unsafe {
        arch::asm!("int {kcall_vector}",
            kcall_vector = const crate::number::KCALL_VECTOR,
            inout("rax") kcall_nr as i64 => ret,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            in("r10") arg3,
            options(nostack, preserves_flags)
        );
    }
    ret
}

//==================================================================================================
// Thread Data Area Helpers
//==================================================================================================

///
/// # Description
///
/// Reads a `u32` value from the Thread Data Area (TDA) via the `%fs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
///
/// # Returns
///
/// The `u32` value stored at `fs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%fs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn read_tda_u32(offset: u32) -> u32 {
    let val: u32;
    unsafe {
        arch::asm!(
            "mov {0:e}, fs:[{1:e}]",
            out(reg) val,
            in(reg) offset,
            options(nostack, readonly, preserves_flags),
        );
    }
    val
}

///
/// # Description
///
/// Writes a `u32` value to the Thread Data Area (TDA) via the `%fs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
/// - `val`: The `u32` value to store at `fs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%fs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn write_tda_u32(offset: u32, val: u32) {
    unsafe {
        arch::asm!(
            "mov fs:[{0:e}], {1:e}",
            in(reg) offset,
            in(reg) val,
            options(nostack, preserves_flags),
        );
    }
}

///
/// # Description
///
/// Atomically replaces a `u32` value in the Thread Data Area (TDA) via the `%fs` segment register.
///
/// # Parameters
///
/// - `offset`: Byte offset within the TDA.
/// - `val`: The new `u32` value to store at `fs:[offset]`.
///
/// # Returns
///
/// The previous `u32` value stored at `fs:[offset]`.
///
/// # Safety
///
/// The caller must ensure:
/// - The `%fs` segment base has been configured via `set_thread_data_area()`.
/// - `offset` refers to a valid, properly aligned `u32` slot within the TDA.
///
#[inline(always)]
pub unsafe fn swap_tda_u32(offset: u32, mut val: u32) -> u32 {
    unsafe {
        arch::asm!(
            "xchg fs:[{0:e}], {1:e}",
            in(reg) offset,
            inout(reg) val,
            options(nostack, preserves_flags),
        );
    }
    val
}
