// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::{
    tm,
    DAY_NAMES,
    MONTH_NAMES,
    TM_YEAR_BASE,
};
use ::sysapi::ffi::c_char;

//==================================================================================================
// Private Functions
//==================================================================================================

/// Writes a byte to the buffer at the given offset and returns the next offset.
unsafe fn write_byte(buf: *mut c_char, offset: usize, byte: u8) -> usize {
    *buf.add(offset) = byte as c_char;
    offset + 1
}

/// Writes a 3-byte name (day or month) to the buffer at the given offset.
unsafe fn write_name(buf: *mut c_char, offset: usize, name: &[u8; 3]) -> usize {
    let mut pos: usize = offset;
    pos = write_byte(buf, pos, name[0]);
    pos = write_byte(buf, pos, name[1]);
    pos = write_byte(buf, pos, name[2]);
    pos
}

/// Writes a 2-digit zero-padded number to the buffer.
unsafe fn write_2digit(buf: *mut c_char, offset: usize, value: i32) -> usize {
    let tens: u8 = ((value / 10) % 10) as u8;
    let ones: u8 = (value % 10) as u8;
    let mut pos: usize = offset;
    pos = write_byte(buf, pos, b'0' + tens);
    pos = write_byte(buf, pos, b'0' + ones);
    pos
}

/// Writes the year (space-padded or sign-prefixed) to the buffer.
unsafe fn write_year(buf: *mut c_char, offset: usize, year: i32) -> usize {
    let mut pos: usize = offset;
    let mut y: i32 = year;

    if y < 0 {
        pos = write_byte(buf, pos, b'-');
        y = -y;
    }

    // Write up to 4 digits.
    let d3: u8 = (y / 1000 % 10) as u8;
    let d2: u8 = (y / 100 % 10) as u8;
    let d1: u8 = (y / 10 % 10) as u8;
    let d0: u8 = (y % 10) as u8;

    pos = write_byte(buf, pos, b'0' + d3);
    pos = write_byte(buf, pos, b'0' + d2);
    pos = write_byte(buf, pos, b'0' + d1);
    pos = write_byte(buf, pos, b'0' + d0);

    pos
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the broken-down time in the `tm` structure to a string of the form:
/// `"Wed Jun 30 21:49:08 1993\n"`.
///
/// # Parameters
///
/// - `timeptr`: Pointer to the broken-down time structure.
/// - `buf`: Pointer to a character buffer of at least 26 bytes.
///
/// # Returns
///
/// On success, returns `buf`. Returns a null pointer if `timeptr` or `buf` is null, or if fields
/// are out of range.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers `timeptr` and `buf`. The caller
/// must ensure `buf` has room for at least 26 bytes.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn asctime_r(timeptr: *const tm, buf: *mut c_char) -> *mut c_char {
    if timeptr.is_null() || buf.is_null() {
        return core::ptr::null_mut();
    }

    let t: &tm = &*timeptr;

    // Validate ranges to avoid out-of-bounds access.
    let wday: usize = if t.tm_wday >= 0 && t.tm_wday <= 6 {
        t.tm_wday as usize
    } else {
        return core::ptr::null_mut();
    };
    let mon: usize = if t.tm_mon >= 0 && t.tm_mon <= 11 {
        t.tm_mon as usize
    } else {
        return core::ptr::null_mut();
    };

    // Validate the remaining numeric fields so out-of-range input fails instead of producing
    // malformed output or overflowing the digit arithmetic below.
    if !(1..=31).contains(&t.tm_mday)
        || !(0..=23).contains(&t.tm_hour)
        || !(0..=59).contains(&t.tm_min)
        || !(0..=60).contains(&t.tm_sec)
    {
        return core::ptr::null_mut();
    }

    // The 26-byte output buffer has a fixed 4-digit year field, so reject years that do not fit it
    // (in particular negative years, which would otherwise write past the end of the buffer).
    let year: i64 = i64::from(t.tm_year) + TM_YEAR_BASE;
    if !(0..=9999).contains(&year) {
        return core::ptr::null_mut();
    }

    let mut pos: usize = 0;

    // Day name.
    pos = write_name(buf, pos, DAY_NAMES[wday]);
    pos = write_byte(buf, pos, b' ');

    // Month name.
    pos = write_name(buf, pos, MONTH_NAMES[mon]);
    pos = write_byte(buf, pos, b' ');

    // Day of month (space-padded).
    if t.tm_mday < 10 {
        pos = write_byte(buf, pos, b' ');
        pos = write_byte(buf, pos, b'0' + t.tm_mday as u8);
    } else {
        pos = write_2digit(buf, pos, t.tm_mday);
    }
    pos = write_byte(buf, pos, b' ');

    // Hour:minute:second.
    pos = write_2digit(buf, pos, t.tm_hour);
    pos = write_byte(buf, pos, b':');
    pos = write_2digit(buf, pos, t.tm_min);
    pos = write_byte(buf, pos, b':');
    pos = write_2digit(buf, pos, t.tm_sec);
    pos = write_byte(buf, pos, b' ');

    // Year.
    pos = write_year(buf, pos, year as i32);
    pos = write_byte(buf, pos, b'\n');
    write_byte(buf, pos, 0);

    buf
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::asctime_r;
    use crate::tm_struct::tm;
    use ::sysapi::ffi::c_char;

    fn buf_to_string(buf: &[c_char; 26]) -> ::std::string::String {
        let bytes: ::std::vec::Vec<u8> = buf
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        ::std::string::String::from_utf8(bytes).expect("valid UTF-8")
    }

    #[test]
    fn test_asctime_r_known_date() {
        // Wednesday, June 30, 1993 21:49:08.
        let t: tm = tm {
            tm_sec: 8,
            tm_min: 49,
            tm_hour: 21,
            tm_mday: 30,
            tm_mon: 5,
            tm_year: 93,
            tm_wday: 3,
            tm_yday: 180,
            tm_isdst: 0,
            tm_gmtoff: 0,
        };
        let mut buf: [c_char; 26] = [0; 26];
        let ret: *mut c_char = unsafe { asctime_r(&t, buf.as_mut_ptr()) };
        assert!(!ret.is_null());
        assert_eq!(buf_to_string(&buf), "Wed Jun 30 21:49:08 1993\n");
    }

    #[test]
    fn test_asctime_r_epoch() {
        let t: tm = tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 1,
            tm_mon: 0,
            tm_year: 70,
            tm_wday: 4,
            tm_yday: 0,
            tm_isdst: 0,
            tm_gmtoff: 0,
        };
        let mut buf: [c_char; 26] = [0; 26];
        let ret: *mut c_char = unsafe { asctime_r(&t, buf.as_mut_ptr()) };
        assert!(!ret.is_null());
        assert_eq!(buf_to_string(&buf), "Thu Jan  1 00:00:00 1970\n");
    }

    #[test]
    fn test_asctime_r_null_pointers() {
        let t: tm = tm::new();
        let mut buf: [c_char; 26] = [0; 26];
        assert!(unsafe { asctime_r(core::ptr::null(), buf.as_mut_ptr()) }.is_null());
        assert!(unsafe { asctime_r(&t, core::ptr::null_mut()) }.is_null());
    }

    #[test]
    fn test_asctime_r_year_out_of_range() {
        // Years that do not fit the fixed 4-digit field must be rejected rather than overflowing
        // the 26-byte buffer.
        let mut t: tm = tm::new();
        t.tm_mday = 1;
        t.tm_wday = 4;

        t.tm_year = -2000; // Year -100 (before year 0).
        let mut buf: [c_char; 26] = [0; 26];
        assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

        t.tm_year = 9000; // Year 10900 (more than 4 digits).
        assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());
    }

    #[test]
    fn test_asctime_r_invalid_fields() {
        // Out-of-range numeric fields must be rejected rather than producing malformed output.
        let base: tm = tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 1,
            tm_mon: 0,
            tm_year: 70,
            tm_wday: 4,
            tm_yday: 0,
            tm_isdst: 0,
            tm_gmtoff: 0,
        };
        let mut buf: [c_char; 26] = [0; 26];

        let mut t: tm = base;
        t.tm_mday = 0;
        assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

        let mut t: tm = base;
        t.tm_hour = 24;
        assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

        let mut t: tm = base;
        t.tm_min = -1;
        assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());

        let mut t: tm = base;
        t.tm_sec = 61;
        assert!(unsafe { asctime_r(&t, buf.as_mut_ptr()) }.is_null());
    }
}
