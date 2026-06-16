// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Fast Byte-Level Memory Set
//==================================================================================================

///
/// # Description
///
/// Performs a fast memory set of `dst` with the byte value `value`.
///
/// # Safety
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `dst` points to a valid memory location that is safe to write to
/// - `dst` is valid for writes of `size` bytes
///
/// # Notes
///
/// If `size == 0`, this function is a no-op.
///
#[inline(always)]
pub(crate) unsafe fn fast_memset(dst: *mut u8, value: u8, size: usize) {
    const QWORD_SIZE: usize = ::core::mem::size_of::<u64>();
    const QWORD_SHIFT: usize = QWORD_SIZE.trailing_zeros() as usize;
    // Replicate the byte into all eight lanes of a 64-bit qword for `rep stosq`.
    // The `stosb` path only reads `al`, which holds `value` as the low byte of `qword`.
    let qword: u64 = (value as u64) * 0x0101_0101_0101_0101;
    // NOTE: Rust's `asm!` implies a full compiler memory barrier by default, so no explicit
    // `lateout("memory")` is needed.
    // NOTE: `rep` with `rcx == 0` performs zero iterations.
    if size & (QWORD_SIZE - 1) == 0 {
        let count: usize = size >> QWORD_SHIFT;
        unsafe {
            core::arch::asm!(
                "cld",
                "rep stosq",
                inout("rdi") dst => _,
                inout("rax") qword => _,
                inout("rcx") count => _,
            );
        }
    } else {
        unsafe {
            core::arch::asm!(
                "cld",
                "rep stosb",
                inout("rdi") dst => _,
                inout("rax") qword => _,
                inout("rcx") size => _,
            );
        }
    }
}
