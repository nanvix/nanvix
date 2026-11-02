// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::strptime;
use crate::tm_struct::tm;
use ::std::ffi::CString;

unsafe fn parse(input: &str, fmt: &str, t: &mut tm) -> isize {
    let s: CString = CString::new(input).unwrap_or_else(|_| CString::default());
    let f: CString = CString::new(fmt).unwrap_or_else(|_| CString::default());
    let ret: *mut i8 = strptime(s.as_ptr(), f.as_ptr(), t);
    if ret.is_null() {
        -1
    } else {
        // Number of bytes consumed from the input.
        ret.cast_const().offset_from(s.as_ptr())
    }
}

#[test]
fn iso_date() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("2021-03-14", "%Y-%m-%d", &mut t) };
    assert_eq!(consumed, 10);
    assert_eq!(t.tm_year, 121);
    assert_eq!(t.tm_mon, 2);
    assert_eq!(t.tm_mday, 14);
}

#[test]
fn abbreviated_month() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("Mar", "%b", &mut t) };
    assert_eq!(consumed, 3);
    assert_eq!(t.tm_mon, 2);
}

#[test]
fn full_month_preferred() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("June", "%B", &mut t) };
    assert_eq!(consumed, 4);
    assert_eq!(t.tm_mon, 5);
}

#[test]
fn time_of_day() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("13:30:45", "%T", &mut t) };
    assert_eq!(consumed, 8);
    assert_eq!(t.tm_hour, 13);
    assert_eq!(t.tm_min, 30);
    assert_eq!(t.tm_sec, 45);
}

#[test]
fn leap_second_allowed() {
    // A seconds value of 60 is accepted to represent a leap second.
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("23:59:60", "%T", &mut t) };
    assert_eq!(consumed, 8);
    assert_eq!(t.tm_sec, 60);
}

#[test]
fn twelve_hour_pm() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("01:30 PM", "%I:%M %p", &mut t) };
    assert_eq!(consumed, 8);
    assert_eq!(t.tm_hour, 13);
    assert_eq!(t.tm_min, 30);
}

#[test]
fn am_pm_before_twelve_hour() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("PM 03", "%p %I", &mut t) };
    assert_eq!(consumed, 5);
    assert_eq!(t.tm_hour, 15);
}

#[test]
fn twelve_hour_midnight() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("12:00 AM", "%I:%M %p", &mut t) };
    assert_eq!(consumed, 8);
    assert_eq!(t.tm_hour, 0);
}

#[test]
fn us_date() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("03/14/21", "%D", &mut t) };
    assert_eq!(consumed, 8);
    assert_eq!(t.tm_mon, 2);
    assert_eq!(t.tm_mday, 14);
    assert_eq!(t.tm_year, 121);
}

#[test]
fn weekday_name() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("Monday", "%A", &mut t) };
    assert_eq!(consumed, 6);
    assert_eq!(t.tm_wday, 1);
}

#[test]
fn space_padded_day() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse(" 7", "%e", &mut t) };
    assert_eq!(consumed, 2);
    assert_eq!(t.tm_mday, 7);
}

#[test]
fn trailing_text_returned() {
    let mut t: tm = tm::new();
    // Only "%Y" is consumed; the remainder is left for the caller.
    let consumed: isize = unsafe { parse("2021 rest", "%Y", &mut t) };
    assert_eq!(consumed, 4);
    assert_eq!(t.tm_year, 121);
}

#[test]
fn century_and_year() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("2021", "%C%y", &mut t) };
    assert_eq!(consumed, 4);
    assert_eq!(t.tm_year, 121);
}

#[test]
fn literal_mismatch_fails() {
    let mut t: tm = tm::new();
    let consumed: isize = unsafe { parse("2021/03", "%Y-%m", &mut t) };
    assert_eq!(consumed, -1);
}

#[test]
fn out_of_range_numeric_fields_fail() {
    // Each specifier must reject values outside its valid range.
    let cases: [(&str, &str); 13] = [
        ("00", "%d"),
        ("32", "%d"),
        ("24", "%H"),
        ("00", "%I"),
        ("13", "%I"),
        ("00", "%m"),
        ("13", "%m"),
        ("367", "%j"),
        ("7", "%w"),
        ("54", "%U"),
        ("54", "%W"),
        ("60", "%M"),
        ("61", "%S"),
    ];
    for (input, fmt) in cases {
        let mut t: tm = tm::new();
        let consumed: isize = unsafe { parse(input, fmt, &mut t) };
        assert_eq!(consumed, -1, "expected {fmt} to reject {input:?}");
    }
}

#[test]
fn null_arguments_fail() {
    let mut t: tm = tm::new();
    let f: CString = CString::new("%Y").unwrap_or_else(|_| CString::default());
    assert!(unsafe { strptime(core::ptr::null(), f.as_ptr(), &mut t) }.is_null());
}
