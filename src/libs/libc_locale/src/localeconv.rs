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

//==================================================================================================
// Structures
//==================================================================================================

/// Locale-specific numeric formatting information, as defined by POSIX.
#[repr(C)]
pub struct lconv {
    /// Decimal-point character for non-monetary values.
    pub decimal_point: *mut c_char,
    /// Thousands separator for non-monetary values.
    pub thousands_sep: *mut c_char,
    /// Grouping sizes for non-monetary values (empty string means no grouping).
    pub grouping: *mut c_char,
    /// International currency symbol.
    pub int_curr_symbol: *mut c_char,
    /// Local currency symbol.
    pub currency_symbol: *mut c_char,
    /// Decimal-point character for monetary values.
    pub mon_decimal_point: *mut c_char,
    /// Thousands separator for monetary values.
    pub mon_thousands_sep: *mut c_char,
    /// Grouping sizes for monetary values.
    pub mon_grouping: *mut c_char,
    /// Positive sign for monetary values.
    pub positive_sign: *mut c_char,
    /// Negative sign for monetary values.
    pub negative_sign: *mut c_char,
    /// Number of fractional digits for international monetary values.
    pub int_frac_digits: c_char,
    /// Number of fractional digits for local monetary values.
    pub frac_digits: c_char,
    /// 1 if `currency_symbol` precedes a positive monetary value.
    pub p_cs_precedes: c_char,
    /// 1 if a space separates `currency_symbol` from a positive monetary value.
    pub p_sep_by_space: c_char,
    /// 1 if `currency_symbol` precedes a negative monetary value.
    pub n_cs_precedes: c_char,
    /// 1 if a space separates `currency_symbol` from a negative monetary value.
    pub n_sep_by_space: c_char,
    /// Positioning of positive sign for monetary values.
    pub p_sign_posn: c_char,
    /// Positioning of negative sign for monetary values.
    pub n_sign_posn: c_char,
    /// 1 if `int_curr_symbol` precedes a positive international monetary value.
    pub int_p_cs_precedes: c_char,
    /// 1 if a space separates `int_curr_symbol` from a positive international monetary value.
    pub int_p_sep_by_space: c_char,
    /// 1 if `int_curr_symbol` precedes a negative international monetary value.
    pub int_n_cs_precedes: c_char,
    /// 1 if a space separates `int_curr_symbol` from a negative international monetary value.
    pub int_n_sep_by_space: c_char,
    /// Positioning of positive sign for international monetary values.
    pub int_p_sign_posn: c_char,
    /// Positioning of negative sign for international monetary values.
    pub int_n_sign_posn: c_char,
}

//==================================================================================================
// Static Data
//==================================================================================================

/// Decimal-point string for the "C" locale.
static DECIMAL_POINT: [u8; 2] = [b'.', 0];

/// Empty string used for locale fields that have no value in the "C" locale.
static EMPTY: [u8; 1] = [0];

/// `CHAR_MAX` sentinel indicating "not available" for numeric locale fields.
const CHAR_MAX: c_char = i8::MAX;

/// Wrapper that makes the immutable `lconv` static shareable across threads.
///
/// `lconv` holds raw pointers and is therefore `!Sync`, but every pointer refers to read-only
/// static data, so sharing the structure across threads is sound.
struct SyncLconv(lconv);

// SAFETY: every pointer in the wrapped `lconv` refers to immutable, read-only static data, so the
// structure is safe to share across threads.
unsafe impl Sync for SyncLconv {}

/// Static `lconv` structure for the "C" locale with POSIX-defined defaults.
static C_LCONV: SyncLconv = SyncLconv(lconv {
    decimal_point: DECIMAL_POINT.as_ptr() as *mut c_char,
    thousands_sep: EMPTY.as_ptr() as *mut c_char,
    grouping: EMPTY.as_ptr() as *mut c_char,
    int_curr_symbol: EMPTY.as_ptr() as *mut c_char,
    currency_symbol: EMPTY.as_ptr() as *mut c_char,
    mon_decimal_point: EMPTY.as_ptr() as *mut c_char,
    mon_thousands_sep: EMPTY.as_ptr() as *mut c_char,
    mon_grouping: EMPTY.as_ptr() as *mut c_char,
    positive_sign: EMPTY.as_ptr() as *mut c_char,
    negative_sign: EMPTY.as_ptr() as *mut c_char,
    int_frac_digits: CHAR_MAX,
    frac_digits: CHAR_MAX,
    p_cs_precedes: CHAR_MAX,
    p_sep_by_space: CHAR_MAX,
    n_cs_precedes: CHAR_MAX,
    n_sep_by_space: CHAR_MAX,
    p_sign_posn: CHAR_MAX,
    n_sign_posn: CHAR_MAX,
    int_p_cs_precedes: CHAR_MAX,
    int_p_sep_by_space: CHAR_MAX,
    int_n_cs_precedes: CHAR_MAX,
    int_n_sep_by_space: CHAR_MAX,
    int_p_sign_posn: CHAR_MAX,
    int_n_sign_posn: CHAR_MAX,
});

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns a pointer to a `lconv` structure containing numeric and monetary formatting
/// information for the current locale.
///
/// # Returns
///
/// A pointer to the static `lconv` structure describing the "C" locale. The structure is immutable
/// and remains valid for the lifetime of the program.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn localeconv() -> *mut lconv {
    core::ptr::addr_of!(C_LCONV.0).cast_mut()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        lconv,
        localeconv,
    };
    use ::sysapi::ffi::c_char;

    #[test]
    fn test_localeconv_returns_non_null() {
        let ret: *mut lconv = localeconv();
        assert!(!ret.is_null());
    }

    #[test]
    fn test_localeconv_decimal_point() {
        let lc: &lconv = unsafe { &*localeconv() };
        assert!(!lc.decimal_point.is_null());
        assert_eq!(
            unsafe { *lc.decimal_point },
            c_char::try_from(b'.').expect("should fit in c_char")
        );
    }

    #[test]
    fn test_localeconv_thousands_sep_empty() {
        let lc: &lconv = unsafe { &*localeconv() };
        assert!(!lc.thousands_sep.is_null());
        assert_eq!(unsafe { *lc.thousands_sep }, 0);
    }

    #[test]
    fn test_localeconv_frac_digits() {
        let lc: &lconv = unsafe { &*localeconv() };
        assert_eq!(lc.frac_digits, i8::MAX);
    }
}
