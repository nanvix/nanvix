// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcscoll_l_orders_like_wcscmp() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    let a: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let b: [wchar_t; 4] = [0x61, 0x62, 0x64, 0];

    assert!(unsafe { wcscoll_l(a.as_ptr(), b.as_ptr(), locale) } < 0);
    assert!(unsafe { wcscoll_l(b.as_ptr(), a.as_ptr(), locale) } > 0);
    assert_eq!(unsafe { wcscoll_l(a.as_ptr(), a.as_ptr(), locale) }, 0);
}

#[test]
fn test_wcsxfrm_l_is_identity_in_c_locale() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    let src: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let mut dest: [wchar_t; 8] = [-1; 8];
    let capacity: c_size_t =
        c_size_t::try_from(dest.len()).expect("destination length should fit in c_size_t");

    let len: c_size_t = unsafe { wcsxfrm_l(dest.as_mut_ptr(), src.as_ptr(), capacity, locale) };
    assert_eq!(len, 3);
    assert_eq!(&dest[..4], &src[..4]);
}
