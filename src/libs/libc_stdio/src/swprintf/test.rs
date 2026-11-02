// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[unsafe(no_mangle)]
unsafe extern "C" fn wcslen(s: *const wchar_t) -> c_size_t {
    let mut len: c_size_t = 0;
    while unsafe { *s.add(len as usize) } != 0 {
        len += 1;
    }
    len
}

#[unsafe(no_mangle)]
unsafe extern "C" fn wcstombs(dst: *mut c_char, src: *const wchar_t, n: c_size_t) -> c_size_t {
    let mut i: c_size_t = 0;
    loop {
        let wc: wchar_t = unsafe { *src.add(i as usize) };
        if wc == 0 {
            if !dst.is_null() && i < n {
                unsafe { *dst.add(i as usize) = 0 };
            }
            return i;
        }
        if !dst.is_null() {
            if i >= n {
                return i;
            }
            unsafe { *dst.add(i as usize) = c_char::from_ne_bytes([(wc & 0xff) as u8]) };
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn mbstowcs(dst: *mut wchar_t, src: *const c_char, n: c_size_t) -> c_size_t {
    let mut i: c_size_t = 0;
    loop {
        let c: c_char = unsafe { *src.add(i as usize) };
        if c == 0 {
            if !dst.is_null() && i < n {
                unsafe { *dst.add(i as usize) = 0 };
            }
            return i;
        }
        if !dst.is_null() {
            if i >= n {
                return i;
            }
            unsafe { *dst.add(i as usize) = wchar_t::from(c.to_ne_bytes()[0]) };
        }
        i += 1;
    }
}

#[test]
fn test_swprintf_fails_when_output_would_not_fit() {
    let mut out: [wchar_t; 2] = [-1; 2];
    let fmt: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let ret: c_int = unsafe { swprintf(out.as_mut_ptr(), out.len() as c_size_t, fmt.as_ptr()) };
    assert_eq!(ret, -1);
}

#[test]
fn test_swprintf_writes_when_output_fits() {
    let mut out: [wchar_t; 4] = [-1; 4];
    let fmt: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let ret: c_int = unsafe { swprintf(out.as_mut_ptr(), out.len() as c_size_t, fmt.as_ptr()) };
    assert_eq!(ret, 3);
    assert_eq!(out, fmt);
}
