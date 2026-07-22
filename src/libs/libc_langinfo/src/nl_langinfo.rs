// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_char;
pub use ::sysapi::nl_types::nl_item;

//==================================================================================================
// Constants
//==================================================================================================

/// Codeset name.
pub const CODESET: nl_item = 0;
/// Preferred date and time format.
pub const D_T_FMT: nl_item = 1;
/// Preferred date format.
pub const D_FMT: nl_item = 2;
/// Preferred time format.
pub const T_FMT: nl_item = 3;
/// Preferred 12-hour time format.
pub const T_FMT_AMPM: nl_item = 4;
/// Ante meridiem string.
pub const AM_STR: nl_item = 5;
/// Post meridiem string.
pub const PM_STR: nl_item = 6;
/// Full name of Sunday.
pub const DAY_1: nl_item = 7;
/// Full name of Monday.
pub const DAY_2: nl_item = 8;
/// Full name of Tuesday.
pub const DAY_3: nl_item = 9;
/// Full name of Wednesday.
pub const DAY_4: nl_item = 10;
/// Full name of Thursday.
pub const DAY_5: nl_item = 11;
/// Full name of Friday.
pub const DAY_6: nl_item = 12;
/// Full name of Saturday.
pub const DAY_7: nl_item = 13;
/// Abbreviated name of Sunday.
pub const ABDAY_1: nl_item = 14;
/// Abbreviated name of Monday.
pub const ABDAY_2: nl_item = 15;
/// Abbreviated name of Tuesday.
pub const ABDAY_3: nl_item = 16;
/// Abbreviated name of Wednesday.
pub const ABDAY_4: nl_item = 17;
/// Abbreviated name of Thursday.
pub const ABDAY_5: nl_item = 18;
/// Abbreviated name of Friday.
pub const ABDAY_6: nl_item = 19;
/// Abbreviated name of Saturday.
pub const ABDAY_7: nl_item = 20;
/// Full name of January.
pub const MON_1: nl_item = 21;
/// Full name of February.
pub const MON_2: nl_item = 22;
/// Full name of March.
pub const MON_3: nl_item = 23;
/// Full name of April.
pub const MON_4: nl_item = 24;
/// Full name of May.
pub const MON_5: nl_item = 25;
/// Full name of June.
pub const MON_6: nl_item = 26;
/// Full name of July.
pub const MON_7: nl_item = 27;
/// Full name of August.
pub const MON_8: nl_item = 28;
/// Full name of September.
pub const MON_9: nl_item = 29;
/// Full name of October.
pub const MON_10: nl_item = 30;
/// Full name of November.
pub const MON_11: nl_item = 31;
/// Full name of December.
pub const MON_12: nl_item = 32;
/// Abbreviated name of January.
pub const ABMON_1: nl_item = 33;
/// Abbreviated name of February.
pub const ABMON_2: nl_item = 34;
/// Abbreviated name of March.
pub const ABMON_3: nl_item = 35;
/// Abbreviated name of April.
pub const ABMON_4: nl_item = 36;
/// Abbreviated name of May.
pub const ABMON_5: nl_item = 37;
/// Abbreviated name of June.
pub const ABMON_6: nl_item = 38;
/// Abbreviated name of July.
pub const ABMON_7: nl_item = 39;
/// Abbreviated name of August.
pub const ABMON_8: nl_item = 40;
/// Abbreviated name of September.
pub const ABMON_9: nl_item = 41;
/// Abbreviated name of October.
pub const ABMON_10: nl_item = 42;
/// Abbreviated name of November.
pub const ABMON_11: nl_item = 43;
/// Abbreviated name of December.
pub const ABMON_12: nl_item = 44;
/// Radix character.
pub const RADIXCHAR: nl_item = 45;
/// Thousands separator.
pub const THOUSEP: nl_item = 46;
/// Expression matching an affirmative response.
pub const YESEXPR: nl_item = 47;
/// Expression matching a negative response.
pub const NOEXPR: nl_item = 48;
/// Currency symbol string.
pub const CRNCYSTR: nl_item = 49;

//==================================================================================================
// Locale Data (C/POSIX locale, UTF-8 codeset)
//==================================================================================================

static DAYS: [&[u8]; 7] = [
    b"Sunday\0",
    b"Monday\0",
    b"Tuesday\0",
    b"Wednesday\0",
    b"Thursday\0",
    b"Friday\0",
    b"Saturday\0",
];

static ABDAYS: [&[u8]; 7] = [
    b"Sun\0", b"Mon\0", b"Tue\0", b"Wed\0", b"Thu\0", b"Fri\0", b"Sat\0",
];

static MONTHS: [&[u8]; 12] = [
    b"January\0",
    b"February\0",
    b"March\0",
    b"April\0",
    b"May\0",
    b"June\0",
    b"July\0",
    b"August\0",
    b"September\0",
    b"October\0",
    b"November\0",
    b"December\0",
];

static ABMONTHS: [&[u8]; 12] = [
    b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0",
    b"Oct\0", b"Nov\0", b"Dec\0",
];

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns locale-specific information for the current (C/POSIX) locale.
///
/// # Parameters
///
/// - `item`: Identifier of the locale item to query, as an `nl_item`.
///
/// # Returns
///
/// A pointer to a null-terminated string describing `item`. Unrecognized items yield an empty
/// string. The codeset is reported as ISO-8859-1 to match the byte-oriented C/POSIX locale
/// multibyte conversions.
///
/// # Safety
///
/// This function is safe for all input values. The returned pointer refers to static, read-only
/// storage and the caller must not modify the pointed-to data.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn nl_langinfo(item: nl_item) -> *mut c_char {
    let s: &'static [u8] = match item {
        CODESET => b"ISO-8859-1\0",
        D_T_FMT => b"%a %b %e %H:%M:%S %Y\0",
        D_FMT => b"%m/%d/%y\0",
        T_FMT => b"%H:%M:%S\0",
        T_FMT_AMPM => b"%I:%M:%S %p\0",
        AM_STR => b"AM\0",
        PM_STR => b"PM\0",
        RADIXCHAR => b".\0",
        THOUSEP => b"\0",
        YESEXPR => b"^[yY]\0",
        NOEXPR => b"^[nN]\0",
        CRNCYSTR => b"\0",
        DAY_1..=DAY_7 => index(&DAYS, item - DAY_1),
        ABDAY_1..=ABDAY_7 => index(&ABDAYS, item - ABDAY_1),
        MON_1..=MON_12 => index(&MONTHS, item - MON_1),
        ABMON_1..=ABMON_12 => index(&ABMONTHS, item - ABMON_1),
        _ => b"\0",
    };
    s.as_ptr() as *mut c_char
}

/// Returns the `idx`-th entry of `table`, or an empty string if `idx` is out of range.
fn index(table: &'static [&'static [u8]], idx: nl_item) -> &'static [u8] {
    match usize::try_from(idx) {
        Ok(i) => match table.get(i) {
            Some(entry) => entry,
            None => b"\0",
        },
        Err(_) => b"\0",
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        nl_item,
        nl_langinfo,
    };
    use ::sysapi::ffi::c_char;

    // Item identifiers mirrored from the implementation for test readability.
    const CODESET: nl_item = 0;
    const D_FMT: nl_item = 2;
    const RADIXCHAR: nl_item = 45;
    const THOUSEP: nl_item = 46;
    const DAY_1: nl_item = 7;
    const DAY_7: nl_item = 13;
    const ABDAY_1: nl_item = 14;
    const MON_1: nl_item = 21;
    const MON_12: nl_item = 32;
    const ABMON_1: nl_item = 33;

    /// Compares a C string returned by `nl_langinfo` against an expected byte slice (no NUL).
    fn c_str_eq(ptr: *mut c_char, expected: &[u8]) -> bool {
        assert!(!ptr.is_null());
        for (i, &byte) in expected.iter().enumerate() {
            let want: c_char = c_char::try_from(byte).expect("ASCII fits in c_char");
            if unsafe { *ptr.add(i) } != want {
                return false;
            }
        }
        // The returned string must terminate right after the expected bytes.
        unsafe { *ptr.add(expected.len()) == 0 }
    }

    #[test]
    fn test_codeset_is_iso_8859_1() {
        assert!(c_str_eq(nl_langinfo(CODESET), b"ISO-8859-1"));
    }

    #[test]
    fn test_date_format() {
        assert!(c_str_eq(nl_langinfo(D_FMT), b"%m/%d/%y"));
    }

    #[test]
    fn test_radixchar_and_thousep() {
        assert!(c_str_eq(nl_langinfo(RADIXCHAR), b"."));
        assert!(c_str_eq(nl_langinfo(THOUSEP), b""));
    }

    #[test]
    fn test_day_names() {
        assert!(c_str_eq(nl_langinfo(DAY_1), b"Sunday"));
        assert!(c_str_eq(nl_langinfo(DAY_7), b"Saturday"));
        assert!(c_str_eq(nl_langinfo(ABDAY_1), b"Sun"));
    }

    #[test]
    fn test_month_names() {
        assert!(c_str_eq(nl_langinfo(MON_1), b"January"));
        assert!(c_str_eq(nl_langinfo(MON_12), b"December"));
        assert!(c_str_eq(nl_langinfo(ABMON_1), b"Jan"));
    }

    #[test]
    fn test_unknown_item_is_empty() {
        assert!(c_str_eq(nl_langinfo(9999), b""));
        assert!(c_str_eq(nl_langinfo(-1), b""));
    }
}
