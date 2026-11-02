// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
