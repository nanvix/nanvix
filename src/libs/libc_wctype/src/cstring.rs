// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Compares a C string pointer against an ASCII byte slice. The byte slice `expected` is not
/// null-terminated; equality requires that `s` matches every byte and is itself terminated by a
/// NUL right after the last compared byte.
pub(crate) unsafe fn c_str_eq(s: *const c_char, expected: &[u8]) -> bool {
    for (i, &byte) in expected.iter().enumerate() {
        let current: c_char = unsafe { *s.add(i) };
        if current == 0 || current.to_ne_bytes()[0] != byte {
            return false;
        }
    }

    unsafe { *s.add(expected.len()) == 0 }
}
