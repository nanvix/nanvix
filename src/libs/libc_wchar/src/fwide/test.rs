// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::fwide;
use ::libc_stdio::FILE;

fn make_file() -> FILE {
    FILE {
        fd: 0,
        eof: 0,
        error: 0,
        ungetc_buf: -1,
        orientation: 0,
    }
}

#[test]
fn sets_wide_orientation_for_positive_mode() {
    let mut stream: FILE = make_file();

    unsafe {
        assert_eq!(fwide(&raw mut stream, 0), 0);
        assert!(fwide(&raw mut stream, 1) > 0);
        assert!(fwide(&raw mut stream, -1) > 0);
        assert!(fwide(&raw mut stream, 0) > 0);
    }
}

#[test]
fn sets_byte_orientation_for_negative_mode() {
    let mut stream: FILE = make_file();

    unsafe {
        assert_eq!(fwide(&raw mut stream, 0), 0);
        assert!(fwide(&raw mut stream, -1) < 0);
        assert!(fwide(&raw mut stream, 1) < 0);
        assert!(fwide(&raw mut stream, 0) < 0);
    }
}

#[test]
fn reports_undetermined_orientation_for_zero_mode() {
    let mut stream: FILE = make_file();

    unsafe {
        assert_eq!(fwide(&raw mut stream, 0), 0);
        assert_eq!(stream.orientation, 0);
    }
}
