// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wchar_t::wchar_t;
use ::sysapi::{
    ffi::{
        c_char,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// External Symbols
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

//==================================================================================================
// Structures
//==================================================================================================

/// Owned narrow copy of the ASCII prefix of a wide string.
pub(crate) struct NarrowString {
    ptr: *mut c_char,
}

impl NarrowString {
    /// Returns a pointer to the null-terminated narrow string.
    pub(crate) fn as_ptr(&self) -> *const c_char {
        self.ptr.cast_const()
    }
}

impl Drop for NarrowString {
    fn drop(&mut self) {
        unsafe { free(self.ptr.cast::<c_void>()) };
    }
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Copies `len` wide single-byte characters to a heap-allocated narrow string.
///
/// # Safety
///
/// `nptr` must point to at least `len` valid wide characters followed by any storage needed by the
/// caller's scan.
unsafe fn copy_narrow_alloc(nptr: *const wchar_t, len: usize) -> Option<NarrowString> {
    let size: c_size_t = c_size_t::try_from(len.saturating_add(1)).ok()?;
    let ptr: *mut c_char = unsafe { malloc(size) }.cast::<c_char>();
    if ptr.is_null() {
        return None;
    }
    for i in 0..len {
        let c: wchar_t = unsafe { *nptr.add(i) };
        let cp: u32 = u32::from_ne_bytes(c.to_ne_bytes());
        let byte: u8 = match u8::try_from(cp) {
            Ok(byte) => byte,
            Err(_) => {
                unsafe { free(ptr.cast::<c_void>()) };
                return None;
            },
        };
        unsafe { *ptr.add(i) = c_char::from_ne_bytes([byte]) };
    }
    unsafe { *ptr.add(len) = 0 };
    Some(NarrowString { ptr })
}

/// Copies the ASCII prefix of a wide numeric string to a heap-allocated narrow string.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string.
pub(crate) unsafe fn to_narrow_alloc(nptr: *const wchar_t) -> Option<NarrowString> {
    let mut len: usize = 0;
    loop {
        let c: wchar_t = unsafe { *nptr.add(len) };
        if c == 0 {
            break;
        }
        let cp: u32 = u32::from_ne_bytes(c.to_ne_bytes());
        if cp > 0x7f {
            break;
        }
        len += 1;
    }

    unsafe { copy_narrow_alloc(nptr, len) }
}

/// Copies a full null-terminated wide string of single-byte characters to a heap-allocated narrow
/// string.
///
/// # Safety
///
/// `nptr` must point to a valid, null-terminated wide string.
pub(crate) unsafe fn to_narrow_alloc_full(nptr: *const wchar_t) -> Option<NarrowString> {
    let mut len: usize = 0;
    loop {
        let c: wchar_t = unsafe { *nptr.add(len) };
        if c == 0 {
            break;
        }
        let cp: u32 = u32::from_ne_bytes(c.to_ne_bytes());
        if cp > u32::from(u8::MAX) {
            return None;
        }
        len += 1;
    }

    unsafe { copy_narrow_alloc(nptr, len) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
