// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::{
    ffi::CString,
    string::String,
    vec,
};

#[test]
fn test_strftime_l_delegates_to_strftime() {
    let mut time: tm = tm::new();
    time.tm_year = 2021 - 1900;
    time.tm_mon = 3 - 1;
    time.tm_mday = 14;

    let format: CString = CString::new("%Y-%m-%d").unwrap_or_else(|_| CString::default());
    let mut buffer: vec::Vec<u8> = vec![0u8; 64];
    let capacity: c_size_t = c_size_t::try_from(buffer.len()).expect("capacity fits");
    let locale: *mut c_void = ::core::ptr::null_mut();

    let written: c_size_t = unsafe {
        strftime_l(buffer.as_mut_ptr().cast::<c_char>(), capacity, format.as_ptr(), &time, locale)
    };

    assert_eq!(String::from_utf8_lossy(&buffer[..written as usize]), "2021-03-14");
}
