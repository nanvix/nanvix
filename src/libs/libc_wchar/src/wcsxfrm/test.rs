// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcsxfrm_copies_identity_transform() {
    let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let mut dest: [wchar_t; 4] = [-1; 4];
    let len: c_size_t = unsafe {
        wcsxfrm(
            dest.as_mut_ptr(),
            src.as_ptr(),
            c_size_t::try_from(dest.len()).expect("destination length should fit in c_size_t"),
        )
    };
    assert_eq!(len, 3);
    assert_eq!(dest, src);
}

#[test]
fn test_wcsxfrm_zero_size_returns_required_length() {
    let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let len: c_size_t = unsafe { wcsxfrm(core::ptr::null_mut(), src.as_ptr(), 0) };
    assert_eq!(len, 3);
}

#[test]
fn test_wcsxfrm_truncates_but_terminates_when_space_exists() {
    let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let mut dest: [wchar_t; 2] = [-1; 2];
    let len: c_size_t = unsafe {
        wcsxfrm(
            dest.as_mut_ptr(),
            src.as_ptr(),
            c_size_t::try_from(dest.len()).expect("destination length should fit in c_size_t"),
        )
    };
    assert_eq!(len, 3);
    assert_eq!(dest, [0x61, 0]);
}
