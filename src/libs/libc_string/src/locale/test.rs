// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::vec::Vec;

fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = bytes
        .iter()
        .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
        .collect();
    v.push(0);
    v
}

#[test]
fn test_strcoll_l_orders_like_strcmp() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    let a: Vec<c_char> = make_c_string(b"abc");
    let b: Vec<c_char> = make_c_string(b"abd");

    assert!(unsafe { strcoll_l(a.as_ptr(), b.as_ptr(), locale) } < 0);
    assert!(unsafe { strcoll_l(b.as_ptr(), a.as_ptr(), locale) } > 0);
    assert_eq!(unsafe { strcoll_l(a.as_ptr(), a.as_ptr(), locale) }, 0);
}

#[test]
fn test_strxfrm_l_is_identity_in_c_locale() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    let src: Vec<c_char> = make_c_string(b"abc");
    let mut dest: [c_char; 8] = [0; 8];
    let capacity: c_size_t = c_size_t::try_from(dest.len()).expect("capacity fits");

    let len: c_size_t = unsafe { strxfrm_l(dest.as_mut_ptr(), src.as_ptr(), capacity, locale) };
    assert_eq!(len, 3);
    assert_eq!(&dest[..4], &src[..4]);
}
