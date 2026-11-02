// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strftime;
use crate::tm_struct::{
    days_from_civil,
    tm,
};
use ::std::{
    ffi::CString,
    string::String,
    vec,
};
use ::sysapi::sys_types::c_size_t;

/// Builds a broken-down time, deriving `tm_wday`/`tm_yday` from the civil date so the tests do
/// not hard-code (and risk mis-stating) those fields.
fn make_tm(year: i32, mon: i32, mday: i32, hour: i32, min: i32, sec: i32) -> tm {
    let mut t: tm = tm::new();
    t.tm_year = year - 1900;
    t.tm_mon = mon - 1;
    t.tm_mday = mday;
    t.tm_hour = hour;
    t.tm_min = min;
    t.tm_sec = sec;

    let days: i64 = days_from_civil(i64::from(year), i64::from(mon), i64::from(mday));
    // 1970-01-01 was a Thursday (weekday 4).
    t.tm_wday = (((days % 7) + 4) % 7 + 7) as i32 % 7;
    let jan1: i64 = days_from_civil(i64::from(year), 1, 1);
    t.tm_yday = (days - jan1) as i32;
    t
}

fn fmt(t: &tm, format: &str) -> String {
    let f: CString = CString::new(format).unwrap_or_else(|_| CString::default());
    let mut buf: vec::Vec<u8> = vec![0u8; 256];
    let n: c_size_t =
        unsafe { strftime(buf.as_mut_ptr().cast::<i8>(), 256 as c_size_t, f.as_ptr(), t) };
    String::from_utf8_lossy(&buf[..n as usize]).into_owned()
}

#[test]
fn date_and_time() {
    let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
    assert_eq!(fmt(&t, "%Y-%m-%d"), "2021-03-14");
    assert_eq!(fmt(&t, "%H:%M:%S"), "09:05:07");
    assert_eq!(fmt(&t, "%F %T"), "2021-03-14 09:05:07");
}

#[test]
fn names() {
    let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
    // 2021-03-14 is a Sunday.
    assert_eq!(fmt(&t, "%a"), "Sun");
    assert_eq!(fmt(&t, "%A"), "Sunday");
    assert_eq!(fmt(&t, "%b"), "Mar");
    assert_eq!(fmt(&t, "%B"), "March");
    assert_eq!(fmt(&t, "%h"), "Mar");
}

#[test]
fn am_pm_and_12hour() {
    let am: tm = make_tm(2021, 3, 14, 9, 49, 8);
    assert_eq!(fmt(&am, "%I:%M:%S %p"), "09:49:08 AM");
    let pm: tm = make_tm(2021, 3, 14, 21, 49, 8);
    assert_eq!(fmt(&pm, "%I:%M:%S %p"), "09:49:08 PM");
    assert_eq!(fmt(&pm, "%H"), "21");
    assert_eq!(fmt(&pm, "%l"), " 9");
    assert_eq!(fmt(&pm, "%P"), "pm");
    let midnight: tm = make_tm(2021, 3, 14, 0, 0, 0);
    assert_eq!(fmt(&midnight, "%I %p"), "12 AM");
}

#[test]
fn numeric_fields() {
    let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
    assert_eq!(fmt(&t, "%j"), "073");
    assert_eq!(fmt(&t, "%y"), "21");
    assert_eq!(fmt(&t, "%C"), "20");
    assert_eq!(fmt(&t, "%w"), "0");
    assert_eq!(fmt(&t, "%u"), "7");
    assert_eq!(fmt(&t, "%e"), "14");
    let single: tm = make_tm(2021, 3, 3, 0, 0, 0);
    assert_eq!(fmt(&single, "%e"), " 3");
    assert_eq!(fmt(&single, "%d"), "03");
}

#[test]
fn two_digit_year_stays_in_range() {
    // Years whose offset from `TM_YEAR_BASE` is negative must still wrap into `00..=99`.
    let mut t: tm = tm::new();
    t.tm_year = -1 - 1900;
    assert_eq!(fmt(&t, "%y"), "99");
}

#[test]
fn literals_and_unknown() {
    let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
    assert_eq!(fmt(&t, "100%% done"), "100% done");
    assert_eq!(fmt(&t, "a%nb%tc"), "a\nb\tc");
    assert_eq!(fmt(&t, "%z %Z"), "+0000 UTC");
}

#[test]
fn truncation_returns_zero() {
    let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
    let f: CString = CString::new("%Y").unwrap_or_else(|_| CString::default());
    let mut buf: vec::Vec<u8> = vec![0u8; 4];
    // "2021" needs 4 bytes plus a null terminator: it cannot fit in 4 bytes.
    let n: c_size_t =
        unsafe { strftime(buf.as_mut_ptr().cast::<i8>(), 4 as c_size_t, f.as_ptr(), &t) };
    assert_eq!(n, 0);
    // A five-byte buffer is exactly enough.
    let mut buf2: vec::Vec<u8> = vec![0u8; 5];
    let n2: c_size_t =
        unsafe { strftime(buf2.as_mut_ptr().cast::<i8>(), 5 as c_size_t, f.as_ptr(), &t) };
    assert_eq!(n2, 4);
}

#[test]
fn zero_capacity_returns_zero() {
    let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
    let f: CString = CString::new("").unwrap_or_else(|_| CString::default());
    let mut buf: [u8; 1] = *b"x";
    let n: c_size_t =
        unsafe { strftime(buf.as_mut_ptr().cast::<i8>(), 0 as c_size_t, f.as_ptr(), &t) };
    assert_eq!(n, 0);
    assert_eq!(buf[0], b'x');
}
