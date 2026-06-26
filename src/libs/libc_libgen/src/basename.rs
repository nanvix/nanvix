// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    c_len,
    dot_ptr,
};
use ::sysapi::ffi::{
    c_char,
    c_uchar,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reports the final component of the path name `path`.
///
/// Trailing `'/'` characters are not counted as part of the path. If `path` consists entirely of
/// `'/'` characters, then `"/"` is returned. If `path` is a null pointer or points to an empty
/// string, then `"."` is returned.
///
/// This function may modify the string pointed to by `path` and may return a pointer into it. When
/// `path` is a null pointer, points to an empty string, or consists entirely of `'/'` characters,
/// the returned pointer may reference read-only storage that the caller must not modify; copy the
/// result first if it needs to be changed. The function is thread-safe.
///
/// # Parameters
///
/// - `path`: The path name to inspect. May be modified in place.
///
/// # Returns
///
/// A pointer to a string that is the final component of `path`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It dereferences the raw pointer `path`.
/// - It performs pointer arithmetic over `path` without bounds checking.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/basename.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn basename(path: *mut c_char) -> *mut c_char {
    if path.is_null() || *path == 0 {
        return dot_ptr();
    }

    let p: *mut c_uchar = path.cast::<c_uchar>();
    let mut i: usize = c_len(p) - 1;

    // Strip trailing slashes, but keep at least the first byte so an all-slash path stays "/".
    while i != 0 && *p.add(i) == b'/' {
        *p.add(i) = 0;
        i -= 1;
    }

    // Back up to the first byte of the final component.
    while i != 0 && *p.add(i - 1) != b'/' {
        i -= 1;
    }

    path.add(i)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::basename;
    use ::std::{
        ffi::CStr,
        string::String,
        vec::Vec,
    };
    use ::sysapi::ffi::c_char;

    /// Builds a mutable, null-terminated C string from `s`.
    fn c_string(s: &str) -> Vec<c_char> {
        let mut v: Vec<c_char> = s
            .bytes()
            .map(|b| c_char::try_from(b).expect("byte fits in c_char"))
            .collect();
        v.push(0);
        v
    }

    /// Reads back the C string returned by `basename` as an owned `String`.
    fn run(s: &str) -> String {
        let mut buf: Vec<c_char> = c_string(s);
        let ret: *mut c_char = unsafe { basename(buf.as_mut_ptr()) };
        let bytes: &[u8] = unsafe { CStr::from_ptr(ret) }.to_bytes();
        String::from_utf8(bytes.to_vec()).expect("valid utf-8")
    }

    #[test]
    fn basename_returns_final_component() {
        assert_eq!(run("/usr/lib"), "lib");
        assert_eq!(run("usr/lib"), "lib");
        assert_eq!(run("lib"), "lib");
    }

    #[test]
    fn basename_handles_roots_and_dots() {
        assert_eq!(run("/"), "/");
        assert_eq!(run("//"), "/");
        assert_eq!(run("///"), "/");
        assert_eq!(run("."), ".");
        assert_eq!(run(".."), "..");
        assert_eq!(run(""), ".");
    }

    #[test]
    fn basename_handles_null_pointer() {
        let ret: *mut c_char = unsafe { basename(::std::ptr::null_mut()) };
        let bytes: &[u8] = unsafe { CStr::from_ptr(ret) }.to_bytes();
        let s: String = String::from_utf8(bytes.to_vec()).expect("valid utf-8");
        assert_eq!(s, ".");
    }

    #[test]
    fn basename_strips_trailing_slashes() {
        assert_eq!(run("/usr/lib/"), "lib");
        assert_eq!(run("/usr/lib//"), "lib");
        assert_eq!(run("usr/"), "usr");
    }

    /// Cases taken verbatim from the POSIX `basename()` sample-input table.
    #[test]
    fn basename_matches_posix_examples() {
        assert_eq!(run("usr"), "usr");
        assert_eq!(run("usr/"), "usr");
        assert_eq!(run("/usr/"), "usr");
        assert_eq!(run("/usr/lib"), "lib");
        assert_eq!(run("//usr//lib//"), "lib");
        assert_eq!(run("/home//dwc//test"), "test");
        assert_eq!(run("/home/.././test"), "test");
        assert_eq!(run("/home/dwc/."), ".");
    }
}
