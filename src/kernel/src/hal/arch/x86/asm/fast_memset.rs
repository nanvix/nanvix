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
    // Replicate the byte into all four lanes of a 32-bit word for `rep stosd`.
    // The `stosb` path only reads `al`, which holds `value` as the low byte of `word`.
    let word: u32 = (value as u32) * 0x0101_0101;
    // NOTE: `esi`/`edi` are saved via push/pop rather than declared as Rust-level clobbers
    // because `asm!` on i686 does not allow naming `esi`/`edi` as operands (they may be reserved
    // by LLVM's register allocator). Manual save/restore is safe on 32-bit x86 (no red zone).
    // NOTE: Rust's `asm!` implies a full compiler memory barrier by default, so no explicit
    // `lateout("memory")` is needed.
    // NOTE: `rep` with `ecx == 0` performs zero iterations.
    if size & (::arch::mem::WORD_SIZE - 1) == 0 {
        let count: usize = size >> ::arch::mem::WORD_SHIFT;
        unsafe {
            core::arch::asm!(
                "push edi",
                "mov edi, {dst}",
                "cld",
                "rep stosd",
                "pop edi",
                dst = in(reg) dst as usize,
                inout("eax") word => _,
                inout("ecx") count => _,
            );
        }
    } else {
        unsafe {
            core::arch::asm!(
                "push edi",
                "mov edi, {dst}",
                "cld",
                "rep stosb",
                "pop edi",
                dst = in(reg) dst as usize,
                inout("eax") word => _,
                inout("ecx") size => _,
            );
        }
    }
}
