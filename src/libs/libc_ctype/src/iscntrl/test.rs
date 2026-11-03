// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::iscntrl;
use ::sysapi::ffi::c_int;

#[test]
fn test_iscntrl_control_chars() {
    for c in 0x00..=0x1F {
        assert_ne!(iscntrl(c), 0, "iscntrl should return non-zero for 0x{:02X}", c);
    }
}

#[test]
fn test_iscntrl_del() {
    assert_ne!(iscntrl(0x7F), 0);
}

#[test]
fn test_iscntrl_non_control() {
    assert_eq!(iscntrl(c_int::from(b' ')), 0);
    assert_eq!(iscntrl(c_int::from(b'A')), 0);
    assert_eq!(iscntrl(c_int::from(b'0')), 0);
    assert_eq!(iscntrl(c_int::from(b'~')), 0);
}

#[test]
fn test_iscntrl_eof() {
    assert_eq!(iscntrl(-1), 0);
}
