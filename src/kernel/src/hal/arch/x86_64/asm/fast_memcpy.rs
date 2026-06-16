// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Fast Byte-Level Memory Copy
//==================================================================================================

///
/// # Description
///
/// Performs a fast memory copy from `src` to `dst`.
///
/// # Safety
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `src` points to a valid memory location that is safe to read from
/// - `dst` points to a valid memory location that is safe to write to
/// - `src` is valid for reads of `size` bytes
/// - `dst` is valid for writes of `size` bytes
/// - `src` and `dst` do not overlap
///
/// # Notes
///
/// If `size == 0`, this function is a no-op.
///
#[inline(always)]
pub(crate) unsafe fn fast_memcpy(dst: *mut u8, src: *const u8, size: usize) {
    const QWORD_SIZE: usize = ::core::mem::size_of::<u64>();
    const QWORD_SHIFT: usize = QWORD_SIZE.trailing_zeros() as usize;
    // NOTE: Rust's `asm!` implies a full compiler memory barrier by default, so no explicit
    // `lateout("memory")` is needed.
    // NOTE: `rep` with `rcx == 0` performs zero iterations.
    if size & (QWORD_SIZE - 1) == 0 {
        let count: usize = size >> QWORD_SHIFT;
        unsafe {
            core::arch::asm!(
                "cld",
                "rep movsq",
                inout("rdi") dst => _,
                inout("rsi") src => _,
                inout("rcx") count => _,
            );
        }
    } else {
        unsafe {
            core::arch::asm!(
                "cld",
                "rep movsb",
                inout("rdi") dst => _,
                inout("rsi") src => _,
                inout("rcx") size => _,
            );
        }
    }
}
