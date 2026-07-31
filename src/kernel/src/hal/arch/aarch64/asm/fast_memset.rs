// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Fills `size` bytes at `dst` with `value`.
///
/// # Safety
///
/// The caller must provide a writable range of at least `size` bytes.
#[inline(always)]
pub(crate) unsafe fn fast_memset(dst: *mut u8, value: u8, size: usize) {
    unsafe { core::ptr::write_bytes(dst, value, size) };
}
