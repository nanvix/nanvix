// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

const STATE_NORMAL: usize = 0;
const STATE_INTEGER: usize = 3;
const STATE_FRACTIONAL: usize = 6;
const STATE_ZERO: usize = 9;

const CLASS_OTHER: usize = 0;
const CLASS_NONZERO_DIGIT: usize = 1;
const CLASS_ZERO: usize = 2;

// States are stored in groups of three so the current byte class can be added to the state base.
const NEXT_STATE: [usize; 12] = [
    STATE_NORMAL,
    STATE_INTEGER,
    STATE_ZERO,
    STATE_NORMAL,
    STATE_INTEGER,
    STATE_INTEGER,
    STATE_NORMAL,
    STATE_FRACTIONAL,
    STATE_FRACTIONAL,
    STATE_NORMAL,
    STATE_FRACTIONAL,
    STATE_ZERO,
];

// Result decisions are indexed by the left state/class and the current right byte class.
const RESULT_TYPE: [Comparison; 36] = [
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::DigitRunLength,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::Fixed(-1),
    Comparison::Fixed(-1),
    Comparison::Fixed(1),
    Comparison::DigitRunLength,
    Comparison::DigitRunLength,
    Comparison::Fixed(1),
    Comparison::DigitRunLength,
    Comparison::DigitRunLength,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::Fixed(1),
    Comparison::Fixed(1),
    Comparison::Fixed(-1),
    Comparison::ByteDiff,
    Comparison::ByteDiff,
    Comparison::Fixed(-1),
    Comparison::ByteDiff,
    Comparison::ByteDiff,
];

//==================================================================================================
// Types
//==================================================================================================

#[derive(Clone, Copy)]
enum Comparison {
    ByteDiff,
    DigitRunLength,
    Fixed(c_int),
}

//==================================================================================================
// Helpers
//==================================================================================================

fn version_class(byte: u8) -> usize {
    match byte {
        b'0' => CLASS_ZERO,
        b'1'..=b'9' => CLASS_NONZERO_DIGIT,
        _ => CLASS_OTHER,
    }
}

unsafe fn compare_digit_run_length(mut s1: *const u8, mut s2: *const u8, fallback: c_int) -> c_int {
    while unsafe { *s1 }.is_ascii_digit() {
        if !unsafe { *s2 }.is_ascii_digit() {
            return 1;
        }

        s1 = unsafe { s1.add(1) };
        s2 = unsafe { s2.add(1) };
    }

    if unsafe { *s2 }.is_ascii_digit() {
        -1
    } else {
        fallback
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Compares two strings treating embedded runs of decimal digits as numbers.
///
/// This GNU extension behaves like `strcmp()` except that aligned runs of decimal digits are
/// compared with version-ordering rules. Non-leading-zero digit runs compare by their remaining
/// length and digit values, so `"file9"` sorts before `"file10"`. Runs with leading zeros sort as
/// fractional components, so `"a00"` sorts before `"a0"` and `"a08"` sorts before `"a8"`.
///
/// # Parameters
///
/// - `s1`: Pointer to the first NUL-terminated string.
/// - `s2`: Pointer to the second NUL-terminated string.
///
/// # Return Value
///
/// An integer less than, equal to, or greater than zero if `s1` is found, respectively, to be
/// less than, to match, or to be greater than `s2`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. It is safe to call if and only if
/// both `s1` and `s2` point to valid NUL-terminated strings.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strverscmp(s1: *const c_char, s2: *const c_char) -> c_int {
    let mut p1: *const u8 = s1.cast();
    let mut p2: *const u8 = s2.cast();

    let mut c1: u8 = unsafe { *p1 };
    let mut c2: u8 = unsafe { *p2 };
    p1 = unsafe { p1.add(1) };
    p2 = unsafe { p2.add(1) };

    let mut state: usize = STATE_NORMAL + version_class(c1);
    loop {
        let diff: c_int = c_int::from(c1) - c_int::from(c2);
        if diff != 0 || c1 == 0 {
            return match RESULT_TYPE[state * 3 + version_class(c2)] {
                Comparison::ByteDiff => diff,
                Comparison::DigitRunLength => unsafe { compare_digit_run_length(p1, p2, diff) },
                Comparison::Fixed(value) => value,
            };
        }

        state = NEXT_STATE[state];
        c1 = unsafe { *p1 };
        c2 = unsafe { *p2 };
        p1 = unsafe { p1.add(1) };
        p2 = unsafe { p2.add(1) };
        state += version_class(c1);
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strverscmp;
    use ::sysapi::ffi::c_char;

    fn cmp(s1: &[u8], s2: &[u8]) -> i32 {
        unsafe { strverscmp(s1.as_ptr().cast::<c_char>(), s2.as_ptr().cast::<c_char>()) }
    }

    #[test]
    fn test_equal_strings() {
        assert_eq!(cmp(b"hello\0", b"hello\0"), 0);
    }

    #[test]
    fn test_numeric_ordering() {
        assert!(cmp(b"file9\0", b"file10\0") < 0);
        assert!(cmp(b"file10\0", b"file9\0") > 0);
    }

    #[test]
    fn test_plain_lexicographic() {
        assert!(cmp(b"abc\0", b"abd\0") < 0);
    }

    #[test]
    fn test_leading_zeros() {
        assert!(cmp(b"a008\0", b"a08\0") < 0);
        assert!(cmp(b"a08\0", b"a008\0") > 0);
        assert!(cmp(b"a0\0", b"a00\0") > 0);
        assert!(cmp(b"a00\0", b"a0\0") < 0);
    }
}
