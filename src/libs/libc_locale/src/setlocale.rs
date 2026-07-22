// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicPtr,
    Ordering,
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Character classification locale category.
pub const LC_CTYPE: c_int = 0;
/// Numeric formatting locale category.
pub const LC_NUMERIC: c_int = 1;
/// Date and time formatting locale category.
pub const LC_TIME: c_int = 2;
/// String collation locale category.
pub const LC_COLLATE: c_int = 3;
/// Monetary formatting locale category.
pub const LC_MONETARY: c_int = 4;
/// Message translation locale category.
pub const LC_MESSAGES: c_int = 5;
/// All locale categories.
pub const LC_ALL: c_int = 6;

/// Mask for the character classification locale category.
pub const LC_CTYPE_MASK: c_int = 1 << LC_CTYPE;
/// Mask for the numeric formatting locale category.
pub const LC_NUMERIC_MASK: c_int = 1 << LC_NUMERIC;
/// Mask for the date and time formatting locale category.
pub const LC_TIME_MASK: c_int = 1 << LC_TIME;
/// Mask for the string collation locale category.
pub const LC_COLLATE_MASK: c_int = 1 << LC_COLLATE;
/// Mask for the monetary formatting locale category.
pub const LC_MONETARY_MASK: c_int = 1 << LC_MONETARY;
/// Mask for the message translation locale category.
pub const LC_MESSAGES_MASK: c_int = 1 << LC_MESSAGES;
/// Mask for all locale categories.
pub const LC_ALL_MASK: c_int = LC_CTYPE_MASK
    | LC_NUMERIC_MASK
    | LC_TIME_MASK
    | LC_COLLATE_MASK
    | LC_MONETARY_MASK
    | LC_MESSAGES_MASK;

/// Static string for the "C" locale name.
static C_LOCALE: [u8; 2] = [b'C', 0];

/// Static string for the "POSIX" locale name.
static POSIX_LOCALE: [u8; 6] = [b'P', b'O', b'S', b'I', b'X', 0];

//==================================================================================================
// Global Variables
//==================================================================================================

/// Pointer to the name of the currently selected locale.
///
/// A null pointer denotes the default `"C"` locale, which is also the initial state.
static CURRENT_LOCALE: AtomicPtr<c_char> = AtomicPtr::new(::core::ptr::null_mut());

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Compares a C string pointer against a byte slice (excluding the null terminator of the slice).
///
/// Returns `true` if the strings match.
unsafe fn c_str_eq(s: *const c_char, expected: &[u8]) -> bool {
    for (i, &byte) in expected.iter().enumerate() {
        let current: c_char = *s.add(i);
        // A NUL here means the C string is shorter than `expected`, so stop before reading past it.
        if current == 0 || current != byte.cast_signed() {
            return false;
        }
    }
    // All expected bytes matched, so reading the terminator is in bounds: the C string ends here.
    *s.add(expected.len()) == 0
}

/// Returns a pointer to the name of the currently selected locale.
///
/// Falls back to the `"C"` locale when no locale has been selected yet.
fn current_locale() -> *mut c_char {
    let current: *mut c_char = CURRENT_LOCALE.load(Ordering::Relaxed);
    if current.is_null() {
        C_LOCALE.as_ptr() as *mut c_char
    } else {
        current
    }
}

///
/// # Description
///
/// Sets the program's current locale. Only the `"C"`, `"POSIX"`, and `""` (empty string,
/// meaning the default "C" locale) locales are supported.
///
/// # Parameters
///
/// - `_category`: The locale category to set (ignored; all categories use "C").
/// - `locale`: Pointer to a null-terminated string naming the desired locale, or a null pointer
///   to query the current locale.
///
/// # Returns
///
/// A pointer to a string identifying the current locale for the given category. Returns a null
/// pointer if the requested locale is not supported.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `locale`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn setlocale(_category: c_int, locale: *const c_char) -> *mut c_char {
    // Query: return the currently selected locale.
    if locale.is_null() {
        return current_locale();
    }

    // Empty string means use the default ("C") locale.
    if *locale == 0 {
        let name: *mut c_char = C_LOCALE.as_ptr() as *mut c_char;
        CURRENT_LOCALE.store(name, Ordering::Relaxed);
        return name;
    }

    // Accept "C".
    if c_str_eq(locale, b"C") {
        let name: *mut c_char = C_LOCALE.as_ptr() as *mut c_char;
        CURRENT_LOCALE.store(name, Ordering::Relaxed);
        return name;
    }

    // Accept "POSIX".
    if c_str_eq(locale, b"POSIX") {
        let name: *mut c_char = POSIX_LOCALE.as_ptr() as *mut c_char;
        CURRENT_LOCALE.store(name, Ordering::Relaxed);
        return name;
    }

    // Unsupported locale: leave the current locale unchanged.
    core::ptr::null_mut()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::setlocale;
    use ::std::sync::{
        Mutex,
        MutexGuard,
    };
    use ::sysapi::ffi::c_char;

    /// Serializes tests that observe the process-global current locale. They mutate and query the
    /// shared `CURRENT_LOCALE`, so they must not run concurrently with one another.
    static SETLOCALE_GUARD: Mutex<()> = Mutex::new(());

    /// Acquires the serialization guard, recovering from poisoning so that a failing test does not
    /// cascade into unrelated ones.
    fn guard() -> MutexGuard<'static, ()> {
        SETLOCALE_GUARD.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn test_setlocale_null_query() {
        let _guard: MutexGuard<'static, ()> = guard();
        // After selecting "C", a NULL query must report "C".
        let c_str: [u8; 2] = [b'C', 0];
        unsafe { setlocale(0, c_str.as_ptr().cast::<c_char>()) };
        let ret: *mut c_char = unsafe { setlocale(0, core::ptr::null()) };
        assert!(!ret.is_null());
        // Should point to "C".
        assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
        assert_eq!(unsafe { *ret.add(1) }, 0);
    }

    #[test]
    fn test_setlocale_empty_string() {
        let _guard: MutexGuard<'static, ()> = guard();
        let empty: [u8; 1] = [0];
        let ret: *mut c_char = unsafe { setlocale(0, empty.as_ptr().cast::<c_char>()) };
        assert!(!ret.is_null());
        assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
    }

    #[test]
    fn test_setlocale_c_locale() {
        let _guard: MutexGuard<'static, ()> = guard();
        let c_str: [u8; 2] = [b'C', 0];
        let ret: *mut c_char = unsafe { setlocale(0, c_str.as_ptr().cast::<c_char>()) };
        assert!(!ret.is_null());
        assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
    }

    #[test]
    fn test_setlocale_posix_locale() {
        let _guard: MutexGuard<'static, ()> = guard();
        let posix: [u8; 6] = [b'P', b'O', b'S', b'I', b'X', 0];
        let ret: *mut c_char = unsafe { setlocale(0, posix.as_ptr().cast::<c_char>()) };
        assert!(!ret.is_null());
        assert_eq!(unsafe { *ret }, c_char::try_from(b'P').expect("should fit in c_char"));
    }

    #[test]
    fn test_setlocale_query_reflects_selection() {
        let _guard: MutexGuard<'static, ()> = guard();
        // Selecting "POSIX" must be observable through a subsequent NULL query.
        let posix: [u8; 6] = [b'P', b'O', b'S', b'I', b'X', 0];
        unsafe { setlocale(0, posix.as_ptr().cast::<c_char>()) };
        let ret: *mut c_char = unsafe { setlocale(0, core::ptr::null()) };
        assert!(!ret.is_null());
        assert_eq!(unsafe { *ret }, c_char::try_from(b'P').expect("should fit in c_char"));

        // Switching back to "C" must likewise be observable.
        let c_str: [u8; 2] = [b'C', 0];
        unsafe { setlocale(0, c_str.as_ptr().cast::<c_char>()) };
        let ret: *mut c_char = unsafe { setlocale(0, core::ptr::null()) };
        assert!(!ret.is_null());
        assert_eq!(unsafe { *ret }, c_char::try_from(b'C').expect("should fit in c_char"));
    }

    #[test]
    fn test_setlocale_unsupported() {
        let _guard: MutexGuard<'static, ()> = guard();
        let utf8: [u8; 6] = [b'e', b'n', b'_', b'U', b'S', 0];
        let ret: *mut c_char = unsafe { setlocale(0, utf8.as_ptr().cast::<c_char>()) };
        assert!(ret.is_null());
    }
}
