// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_floating_l_delegates() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    let nptr: *const c_char = c"3.5".as_ptr();
    let null_end: *mut *mut c_char = ::core::ptr::null_mut();

    assert_eq!(unsafe { strtod_l(nptr, null_end, locale) }, 3.5);
    assert_eq!(unsafe { strtof_l(nptr, null_end, locale) }, 3.5f32);
    assert_eq!(unsafe { strtold_l(nptr, null_end, locale) }, 3.5);
}

#[test]
fn test_integer_l_delegates() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    let signed: *const c_char = c"-42".as_ptr();
    let unsigned: *const c_char = c"100".as_ptr();
    let null_end: *mut *mut c_char = ::core::ptr::null_mut();

    assert_eq!(unsafe { strtol_l(signed, null_end, 10, locale) }, -42);
    assert_eq!(unsafe { strtoll_l(signed, null_end, 10, locale) }, -42);
    assert_eq!(unsafe { strtoul_l(unsigned, null_end, 10, locale) }, 100);
    assert_eq!(unsafe { strtoull_l(unsigned, null_end, 10, locale) }, 100);
}

#[test]
fn test_endptr_is_updated() {
    let locale: *mut c_void = ::core::ptr::null_mut();
    let nptr: *const c_char = c"42rest".as_ptr();
    let mut end: *mut c_char = ::core::ptr::null_mut();

    let value: c_long = unsafe { strtol_l(nptr, &mut end, 10, locale) };
    assert_eq!(value, 42);
    // The parse consumes "42" and stops at the 'r' (index 2).
    assert_eq!(end.cast_const(), unsafe { nptr.add(2) });
}
