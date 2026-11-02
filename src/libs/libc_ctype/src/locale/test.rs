// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_classification_l_delegates() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    assert_ne!(isdigit_l(i32::from(b'7'), locale), 0);
    assert_eq!(isdigit_l(i32::from(b'a'), locale), 0);
    assert_ne!(isalpha_l(i32::from(b'a'), locale), 0);
    assert_ne!(isupper_l(i32::from(b'A'), locale), 0);
    assert_ne!(islower_l(i32::from(b'a'), locale), 0);
    assert_ne!(isspace_l(i32::from(b' '), locale), 0);
    assert_ne!(isxdigit_l(i32::from(b'f'), locale), 0);
    assert_eq!(isxdigit_l(i32::from(b'g'), locale), 0);
}

#[test]
fn test_conversion_l_delegates() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    assert_eq!(tolower_l(i32::from(b'A'), locale), i32::from(b'a'));
    assert_eq!(toupper_l(i32::from(b'a'), locale), i32::from(b'A'));
}
