// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::{
    align_of,
    size_of,
};
use ::sysapi::{
    ffi::{
        c_int,
        c_uchar,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sets bytes in memory to a specified value.
///
/// This function fills the first `len` bytes of the memory area pointed to by `ptr` with the
/// constant byte `val` (converted to an unsigned char).
///
/// # Parameters
///
/// - `ptr`: Pointer to the memory area to be filled.
/// - `val`: Value to be set. Only the least significant byte is used.
/// - `len`: Number of bytes to be set.
///
/// # Return Value
///
/// This function returns a pointer to the memory area `ptr`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `ptr`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `ptr` points to a valid and writable memory region of at least `len` bytes.
/// - `len` does not exceed `isize::MAX`.
/// - `ptr` is properly aligned for `c_uchar` access.
///
/// Violating any of these conditions results in undefined behavior.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memset(ptr: *mut c_void, val: c_int, len: c_size_t) -> *mut c_void {
    debug_assert!(!ptr.is_null(), "memset(): null pointer");
    debug_assert!((len as usize) < isize::MAX as usize, "memset(): length too large");
    debug_assert!(
        (ptr as usize).is_multiple_of(align_of::<c_uchar>()),
        "memset(): pointer is not properly aligned"
    );

    let dst: *mut c_uchar = ptr.cast::<c_uchar>();
    let byte: c_uchar = (val & 0xFF) as c_uchar;
    let len: usize = len as usize;

    const WORD_SIZE: usize = size_of::<usize>();
    const WORD_ALIGN: usize = align_of::<usize>();
    let d_addr: usize = dst as usize;
    // Broadcast the fill byte across a full machine word (0x01010101... * byte).
    let word: usize = (byte as usize).wrapping_mul(usize::MAX / 0xFF);

    // Fill leading bytes until the destination is word-aligned.
    let mut offset: usize = 0;
    while ((d_addr + offset) & (WORD_ALIGN - 1)) != 0 && offset < len {
        *dst.add(offset) = byte;
        offset += 1;
    }
    // Bulk fill a word at a time.
    let mut i: usize = offset;
    while i + WORD_SIZE <= len {
        *dst.add(i).cast::<usize>() = word;
        i += WORD_SIZE;
    }
    // Fill the trailing bytes.
    while i < len {
        *dst.add(i) = byte;
        i += 1;
    }
    ptr
}

#[cfg(all(test, feature = "std"))]
mod test;
