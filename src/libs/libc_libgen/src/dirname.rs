// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    c_len,
    dot_ptr,
    slash_ptr,
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
/// Reports the parent directory portion of the path name `path`.
///
/// Trailing `'/'` characters are not counted as part of the path. If `path` does not contain a
/// `'/'`, then `"."` is returned. If `path` is a null pointer or points to an empty string, then
/// `"."` is returned.
///
/// This function may modify the string pointed to by `path` and may return a pointer into it. When
/// `path` is a null pointer, points to an empty string, contains no `'/'`, or names the root
/// directory, the returned pointer may reference read-only storage that the caller must not modify;
/// copy the result first if it needs to be changed. The function is thread-safe.
///
/// # Parameters
///
/// - `path`: The path name to inspect. May be modified in place.
///
/// # Returns
///
/// A pointer to a string that is the parent directory of `path`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It dereferences the raw pointer `path`.
/// - It performs pointer arithmetic over `path` without bounds checking.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/dirname.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn dirname(path: *mut c_char) -> *mut c_char {
    if path.is_null() || *path == 0 {
        return dot_ptr();
    }

    let p: *mut c_uchar = path.cast::<c_uchar>();
    let mut i: usize = c_len(p) - 1;

    // Strip trailing slashes; a path consisting solely of slashes resolves to the root.
    loop {
        if *p.add(i) != b'/' {
            break;
        }
        if i == 0 {
            return slash_ptr();
        }
        i -= 1;
    }

    // Skip the trailing (basename) component; with no separator the directory is the cwd.
    loop {
        if *p.add(i) == b'/' {
            break;
        }
        if i == 0 {
            return dot_ptr();
        }
        i -= 1;
    }

    // Strip the slash(es) separating the directory from the basename; all slashes means root.
    loop {
        if *p.add(i) != b'/' {
            break;
        }
        if i == 0 {
            return slash_ptr();
        }
        i -= 1;
    }

    *p.add(i + 1) = 0;
    path
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::dirname;
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

    /// Reads back the C string returned by `dirname` as an owned `String`.
    fn run(s: &str) -> String {
        let mut buf: Vec<c_char> = c_string(s);
        let ret: *mut c_char = unsafe { dirname(buf.as_mut_ptr()) };
        let bytes: &[u8] = unsafe { CStr::from_ptr(ret) }.to_bytes();
        String::from_utf8(bytes.to_vec()).expect("valid utf-8")
    }

    #[test]
    fn dirname_returns_parent_directory() {
        assert_eq!(run("/usr/lib"), "/usr");
        assert_eq!(run("/usr/"), "/");
        assert_eq!(run("usr/lib"), "usr");
    }

    #[test]
    fn dirname_handles_roots_and_dots() {
        assert_eq!(run("/"), "/");
        assert_eq!(run("//"), "/");
        assert_eq!(run("///"), "/");
        assert_eq!(run("usr"), ".");
        assert_eq!(run("."), ".");
        assert_eq!(run(".."), ".");
        assert_eq!(run(""), ".");
    }

    #[test]
    fn dirname_handles_null_pointer() {
        let ret: *mut c_char = unsafe { dirname(::std::ptr::null_mut()) };
        let bytes: &[u8] = unsafe { CStr::from_ptr(ret) }.to_bytes();
        let s: String = String::from_utf8(bytes.to_vec()).expect("valid utf-8");
        assert_eq!(s, ".");
    }

    #[test]
    fn dirname_strips_trailing_slashes() {
        assert_eq!(run("/usr/lib/"), "/usr");
        assert_eq!(run("/a//b"), "/a");
        assert_eq!(run("a/b/"), "a");
    }

    /// Cases taken from the POSIX `dirname()` sample-input table. Where the standard permits more
    /// than one result, the value produced by this implementation is asserted.
    #[test]
    fn dirname_matches_posix_examples() {
        assert_eq!(run("usr/"), ".");
        assert_eq!(run("/usr/"), "/");
        assert_eq!(run("/usr/lib"), "/usr");
        assert_eq!(run("//usr//lib//"), "//usr");
        assert_eq!(run("/home//dwc//test"), "/home//dwc");
        assert_eq!(run("/home/.././test"), "/home/../.");
        assert_eq!(run("/home/dwc/."), "/home/dwc");
    }
}
