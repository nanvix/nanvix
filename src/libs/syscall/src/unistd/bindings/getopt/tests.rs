// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    diagnostic_kind,
    GetoptDiagnostic,
};
use ::std::ffi::CString;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

/// Returns the option character `c` as the [`c_int`] value `getopt` reports.
fn opt(c: u8) -> c_int {
    c_int::from(c)
}

/// Returns the diagnostic decision for one synthetic `getopt()` result.
fn diagnostic(
    rc: u8,
    option: u8,
    optstring: &str,
    opterr_value: c_int,
) -> Option<GetoptDiagnostic> {
    let optstring: CString = CString::new(optstring).expect("no interior nul");
    unsafe {
        diagnostic_kind(opt(rc), opt(option), optstring.as_ptr().cast::<c_char>(), opterr_value)
    }
}

#[test]
fn diagnostic_reports_unknown_option() {
    assert_eq!(diagnostic(b'?', b'x', "a", 1), Some(GetoptDiagnostic::IllegalOption));
}

#[test]
fn diagnostic_reports_missing_argument() {
    assert_eq!(diagnostic(b'?', b'a', "a:", 1), Some(GetoptDiagnostic::MissingArgument));
}

#[test]
fn diagnostic_treats_colon_option_as_illegal_option() {
    assert_eq!(diagnostic(b'?', b':', "a::", 1), Some(GetoptDiagnostic::IllegalOption));
}

#[test]
fn diagnostic_is_suppressed_when_opterr_is_zero() {
    assert_eq!(diagnostic(b'?', b'x', "a", 0), None);
}

#[test]
fn diagnostic_is_suppressed_by_leading_colon() {
    assert_eq!(diagnostic(b'?', b'x', ":a", 1), None);
}

#[test]
fn diagnostic_is_suppressed_by_leading_plus_then_colon() {
    assert_eq!(diagnostic(b'?', b'a', "+:a:", 1), None);
}

#[test]
fn diagnostic_is_not_reported_for_successful_option() {
    assert_eq!(diagnostic(b'a', b'a', "a", 1), None);
}
