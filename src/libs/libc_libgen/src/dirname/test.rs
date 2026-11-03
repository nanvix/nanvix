// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
