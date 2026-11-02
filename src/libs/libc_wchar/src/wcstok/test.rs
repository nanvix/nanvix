// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_wcstok_basic() {
    // "a,b,c" tokenized on ",".
    let mut buf: [wchar_t; 6] = [0x61, 0x2C, 0x62, 0x2C, 0x63, 0];
    let delim: [wchar_t; 2] = [0x2C, 0];
    let mut state: *mut wchar_t = core::ptr::null_mut();

    let t1: *mut wchar_t = unsafe { wcstok(buf.as_mut_ptr(), delim.as_ptr(), &mut state) };
    assert!(!t1.is_null());
    assert_eq!(unsafe { *t1 }, 0x61);

    let t2: *mut wchar_t = unsafe { wcstok(core::ptr::null_mut(), delim.as_ptr(), &mut state) };
    assert!(!t2.is_null());
    assert_eq!(unsafe { *t2 }, 0x62);

    let t3: *mut wchar_t = unsafe { wcstok(core::ptr::null_mut(), delim.as_ptr(), &mut state) };
    assert!(!t3.is_null());
    assert_eq!(unsafe { *t3 }, 0x63);

    let t4: *mut wchar_t = unsafe { wcstok(core::ptr::null_mut(), delim.as_ptr(), &mut state) };
    assert!(t4.is_null());
}

#[test]
fn test_wcstok_leading_delimiters() {
    // Leading delimiters are skipped before the first token.
    let mut buf: [wchar_t; 5] = [0x2C, 0x2C, 0x61, 0x62, 0];
    let delim: [wchar_t; 2] = [0x2C, 0];
    let mut state: *mut wchar_t = core::ptr::null_mut();

    let t1: *mut wchar_t = unsafe { wcstok(buf.as_mut_ptr(), delim.as_ptr(), &mut state) };
    assert!(!t1.is_null());
    assert_eq!(unsafe { *t1 }, 0x61);
    assert_eq!(unsafe { *t1.add(1) }, 0x62);
}

#[test]
fn test_wcstok_all_delimiters() {
    // A string made up solely of delimiters yields no tokens.
    let mut buf: [wchar_t; 3] = [0x2C, 0x2C, 0];
    let delim: [wchar_t; 2] = [0x2C, 0];
    let mut state: *mut wchar_t = core::ptr::null_mut();
    let t1: *mut wchar_t = unsafe { wcstok(buf.as_mut_ptr(), delim.as_ptr(), &mut state) };
    assert!(t1.is_null());
}
