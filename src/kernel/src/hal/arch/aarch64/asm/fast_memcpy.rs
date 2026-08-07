// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Copies `size` non-overlapping bytes from `src` to `dst`.
///
/// # Safety
///
/// The caller must provide readable and writable non-overlapping ranges of at least `size` bytes.
#[inline(always)]
pub(crate) unsafe fn fast_memcpy(dst: *mut u8, src: *const u8, size: usize) {
    unsafe { core::ptr::copy_nonoverlapping(src, dst, size) };
}
