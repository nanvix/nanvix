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
                "push esi",
                "push edi",
                "mov edi, {dst}",
                "mov esi, {src}",
                "cld",
                "rep movsd",
                "pop edi",
                "pop esi",
                dst = in(reg) dst as usize,
                src = in(reg) src as usize,
                inout("ecx") count => _,
            );
        }
    } else {
        unsafe {
            core::arch::asm!(
                "push esi",
                "push edi",
                "mov edi, {dst}",
                "mov esi, {src}",
                "cld",
                "rep movsb",
                "pop edi",
                "pop esi",
                dst = in(reg) dst as usize,
                src = in(reg) src as usize,
                inout("ecx") size => _,
            );
        }
    }
}
