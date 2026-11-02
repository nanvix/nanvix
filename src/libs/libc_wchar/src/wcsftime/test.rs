// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::sysapi::ffi::c_int;

#[repr(C)]
struct HostTm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
}

fn make_time() -> HostTm {
    HostTm {
        tm_sec: 7,
        tm_min: 5,
        tm_hour: 9,
        tm_mday: 14,
        tm_mon: 2,
        tm_year: 121,
        tm_wday: 0,
        tm_yday: 72,
        tm_isdst: 0,
    }
}

fn c_size_len(len: usize) -> c_size_t {
    c_size_t::try_from(len).expect("buffer length should fit in c_size_t")
}

#[test]
fn test_wcsftime_formats_time() {
    let mut dest: [wchar_t; 5] = [0; 5];
    let format: [wchar_t; 3] = [0x25, 0x59, 0];
    let time: HostTm = make_time();

    let written: c_size_t = unsafe {
        wcsftime(
            dest.as_mut_ptr(),
            c_size_len(dest.len()),
            format.as_ptr(),
            (&time as *const HostTm).cast::<tm>(),
        )
    };

    assert_eq!(written, 4);
    assert_eq!(dest, [0x32, 0x30, 0x32, 0x31, 0]);
}

#[test]
fn test_wcsftime_empty_format_terminates_destination() {
    let mut dest: [wchar_t; 1] = [0x78];
    let format: [wchar_t; 1] = [0];
    let time: HostTm = make_time();

    let written: c_size_t = unsafe {
        wcsftime(
            dest.as_mut_ptr(),
            c_size_len(dest.len()),
            format.as_ptr(),
            (&time as *const HostTm).cast::<tm>(),
        )
    };

    assert_eq!(written, 0);
    assert_eq!(dest[0], 0);
}

#[test]
fn test_wcsftime_rejects_unrepresentable_format_character() {
    let mut dest: [wchar_t; 4] = [-1; 4];
    let format: [wchar_t; 4] = [0x58, 0x100, 0x59, 0];
    let time: HostTm = make_time();

    let written: c_size_t = unsafe {
        wcsftime(
            dest.as_mut_ptr(),
            c_size_len(dest.len()),
            format.as_ptr(),
            (&time as *const HostTm).cast::<tm>(),
        )
    };

    assert_eq!(written, 0);
    assert_eq!(dest, [-1; 4]);
}
