// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mbstate::mbstate_t,
    wchar_t::wchar_t,
};
use ::sysapi::{
    errno::{
        __errno_location,
        EILSEQ,
    },
    ffi::{
        c_char,
        c_int,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Return value used by the byte-counting functions to signal an error.
const SIZE_ERR: usize = usize::MAX;

/// Return value used by the restartable functions to signal an incomplete sequence.
const SIZE_INCOMPLETE: usize = usize::MAX - 1;

//==================================================================================================
// Internal Conversion State
//==================================================================================================

/// Per-thread internal conversion state used by the restartable functions when the caller passes a
/// null `mbstate_t`.
///
/// On the host `std` build the unit-test harness runs tests on multiple threads, so the implicit
/// state is kept thread-local to avoid data races. On the guest `no_std` build there is no
/// thread-local support, so a process-global state is used instead, matching the conventional
/// single-threaded C library behavior.
#[cfg(feature = "std")]
mod internal_state {
    use crate::mbstate::mbstate_t;
    use ::core::cell::Cell;

    std::thread_local! {
        static MBRTOWC_STATE: Cell<mbstate_t> =
            const { Cell::new(mbstate_t { count: 0, bytes: [0; 4] }) };
        static MBRLEN_STATE: Cell<mbstate_t> =
            const { Cell::new(mbstate_t { count: 0, bytes: [0; 4] }) };
    }

    /// Returns a pointer to the thread-local internal state used by `mbrtowc()`.
    pub(super) fn mbrtowc_state() -> *mut mbstate_t {
        MBRTOWC_STATE.with(Cell::as_ptr)
    }

    /// Returns a pointer to the thread-local internal state used by `mbrlen()`.
    pub(super) fn mbrlen_state() -> *mut mbstate_t {
        MBRLEN_STATE.with(Cell::as_ptr)
    }
}

#[cfg(not(feature = "std"))]
mod internal_state {
    use crate::mbstate::mbstate_t;

    /// Internal state used by `mbrtowc()` when the caller passes a null `mbstate_t`.
    static mut MBRTOWC_STATE: mbstate_t = mbstate_t {
        count: 0,
        bytes: [0; 4],
    };

    /// Internal state used by `mbrlen()` when the caller passes a null `mbstate_t`.
    static mut MBRLEN_STATE: mbstate_t = mbstate_t {
        count: 0,
        bytes: [0; 4],
    };

    /// Returns a pointer to the process-global internal state used by `mbrtowc()`.
    pub(super) fn mbrtowc_state() -> *mut mbstate_t {
        ::core::ptr::addr_of_mut!(MBRTOWC_STATE)
    }

    /// Returns a pointer to the process-global internal state used by `mbrlen()`.
    pub(super) fn mbrlen_state() -> *mut mbstate_t {
        ::core::ptr::addr_of_mut!(MBRLEN_STATE)
    }
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Sets `errno` to `code`.
fn set_errno(code: c_int) {
    // SAFETY: `__errno_location()` returns a valid pointer to `errno`.
    unsafe {
        *__errno_location() = code;
    }
}

/// Reinterprets a raw `char` byte as an unsigned byte without a sign-changing `as` cast.
fn byte_of(c: c_char) -> u8 {
    c.to_ne_bytes()[0]
}

/// Reinterprets an unsigned byte as the platform `char` type without a sign-changing `as` cast.
fn cchar_of(b: u8) -> c_char {
    c_char::from_ne_bytes([b])
}

/// Reinterprets a wide character as an unsigned code point.
fn cp_of(wc: wchar_t) -> u32 {
    u32::from_ne_bytes(wc.to_ne_bytes())
}

/// Core of the restartable multibyte-to-wide conversion. Nanvix currently supports the C/POSIX
/// locale, where every byte value is a valid single-byte character.
///
/// # Safety
///
/// `s` (when non-null) must reference at least `n` valid bytes, and `pwc` (when non-null) must be
/// writable.
unsafe fn mbrtowc_core(
    pwc: *mut wchar_t,
    s: *const c_char,
    n: usize,
    state: &mut mbstate_t,
) -> usize {
    if s.is_null() {
        state.count = 0;
        return 0;
    }

    state.count = 0;
    if n == 0 {
        return SIZE_INCOMPLETE;
    }

    let b: u8 = byte_of(unsafe { *s });
    if !pwc.is_null() {
        unsafe { *pwc = wchar_t::from(b) };
    }
    if b == 0 {
        0
    } else {
        1
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Determines the number of bytes in the multibyte character pointed to by `s`, storing the wide
/// character in `*pwc`. The C/POSIX locale is stateless and maps bytes directly to wide values.
///
/// # Safety
///
/// `s` (when non-null) must reference at least `n` valid bytes, and `pwc` (when non-null) must be
/// writable.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mbtowc(pwc: *mut wchar_t, s: *const c_char, n: usize) -> c_int {
    if s.is_null() {
        return 0;
    }

    if n == 0 {
        return -1;
    }

    let b: u8 = byte_of(unsafe { *s });
    if !pwc.is_null() {
        unsafe { *pwc = wchar_t::from(b) };
    }
    if b == 0 {
        0
    } else {
        1
    }
}

/// Determines the number of bytes in the multibyte character pointed to by `s`. Equivalent to
/// `mbtowc(NULL, s, n)` for the byte-oriented C/POSIX locale, where each non-null byte is a
/// complete single-byte character.
///
/// # Safety
///
/// `s` (when non-null) must reference at least `n` valid bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mblen(s: *const c_char, n: usize) -> c_int {
    unsafe { mbtowc(core::ptr::null_mut(), s, n) }
}

/// Stores the multibyte representation of `wc` in `s`, returning the number of bytes written.
///
/// # Safety
///
/// `s` (when non-null) must have room for at least `MB_LEN_MAX` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wctomb(s: *mut c_char, wc: wchar_t) -> c_int {
    if s.is_null() {
        return 0;
    }

    let cp: u32 = cp_of(wc);
    if cp <= 0xff {
        unsafe { *s = cchar_of(u8::try_from(cp).unwrap_or(0)) };
        1
    } else {
        set_errno(EILSEQ);
        -1
    }
}

/// Converts the multibyte string `src` to a wide-character string `dst`, writing at most `n` wide
/// characters. Returns the number of wide characters (excluding the terminator), or `(size_t)-1`
/// on an invalid sequence.
///
/// # Safety
///
/// `src` must be a valid, null-terminated multibyte string and `dst` (when non-null) must have
/// room for `n` wide characters.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mbstowcs(dst: *mut wchar_t, src: *const c_char, n: usize) -> usize {
    let mut s: *const c_char = src;
    let mut count: usize = 0;

    loop {
        if !dst.is_null() && count >= n {
            return count;
        }

        let b: u8 = byte_of(unsafe { *s });
        if b == 0 {
            if !dst.is_null() {
                unsafe { *dst.add(count) = 0 };
            }
            return count;
        }
        if !dst.is_null() {
            unsafe { *dst.add(count) = wchar_t::from(b) };
        }
        count += 1;
        s = unsafe { s.add(1) };
    }
}

/// Converts the wide-character string `src` to a multibyte string `dst`, writing at most `n`
/// bytes. Returns the number of bytes (excluding the terminator), or `(size_t)-1` on an invalid
/// wide character.
///
/// # Safety
///
/// `src` must be a valid, null-terminated wide string and `dst` (when non-null) must have room for
/// `n` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcstombs(dst: *mut c_char, src: *const wchar_t, n: usize) -> usize {
    let mut count: usize = 0;
    let mut i: usize = 0;

    loop {
        let wc: wchar_t = unsafe { *src.add(i) };
        if wc == 0 {
            if !dst.is_null() && count < n {
                unsafe { *dst.add(count) = 0 };
            }
            return count;
        }

        let cp: u32 = cp_of(wc);
        if cp > 0xff {
            set_errno(EILSEQ);
            return SIZE_ERR;
        }
        if dst.is_null() {
            count += 1;
        } else {
            if count >= n {
                return count;
            }
            unsafe { *dst.add(count) = cchar_of(u8::try_from(cp).unwrap_or(0)) };
            count += 1;
        }
        i += 1;
    }
}

/// Restartable conversion of a single multibyte character to a wide character.
///
/// # Safety
///
/// See [`mbrtowc_core`].
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mbrtowc(
    pwc: *mut wchar_t,
    s: *const c_char,
    n: usize,
    ps: *mut mbstate_t,
) -> usize {
    if ps.is_null() {
        // SAFETY: the internal state pointer is valid and the state is only accessed from this
        // function (per-thread under `std`, process-global under `no_std`).
        unsafe { mbrtowc_core(pwc, s, n, &mut *internal_state::mbrtowc_state()) }
    } else {
        unsafe { mbrtowc_core(pwc, s, n, &mut *ps) }
    }
}

/// Restartable computation of the length in bytes of the next multibyte character.
///
/// # Safety
///
/// `s` (when non-null) must reference at least `n` valid bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mbrlen(s: *const c_char, n: usize, ps: *mut mbstate_t) -> usize {
    if ps.is_null() {
        // SAFETY: the internal state pointer is valid and the state is only accessed from this
        // function (per-thread under `std`, process-global under `no_std`).
        unsafe { mbrtowc_core(core::ptr::null_mut(), s, n, &mut *internal_state::mbrlen_state()) }
    } else {
        unsafe { mbrtowc_core(core::ptr::null_mut(), s, n, &mut *ps) }
    }
}

/// Restartable conversion of a single wide character to its multibyte representation. The
/// byte-oriented C/POSIX locale is single-byte and stateless, so `ps` is ignored apart from
/// honouring the null-`s` reset convention.
///
/// # Safety
///
/// `s` (when non-null) must have room for at least `MB_LEN_MAX` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcrtomb(s: *mut c_char, wc: wchar_t, _ps: *mut mbstate_t) -> usize {
    // With a null buffer the call is equivalent to encoding a null wide character.
    if s.is_null() {
        return 1;
    }

    let cp: u32 = cp_of(wc);
    if cp <= 0xff {
        unsafe { *s = cchar_of(u8::try_from(cp).unwrap_or(0)) };
        1
    } else {
        set_errno(EILSEQ);
        SIZE_ERR
    }
}

/// Reports whether `ps` describes an initial conversion state.
///
/// # Safety
///
/// `ps` (when non-null) must be a valid pointer.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mbsinit(ps: *const mbstate_t) -> c_int {
    if ps.is_null() {
        return 1;
    }
    if unsafe { (*ps).count } == 0 {
        1
    } else {
        0
    }
}

/// Restartable conversion of a multibyte string to a wide-character string.
///
/// # Safety
///
/// `src` must point to a valid pointer to a null-terminated multibyte string. `dst` (when
/// non-null) must have room for `n` wide characters.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mbsrtowcs(
    dst: *mut wchar_t,
    src: *mut *const c_char,
    n: usize,
    ps: *mut mbstate_t,
) -> usize {
    let mut state: mbstate_t = if ps.is_null() {
        mbstate_t {
            count: 0,
            bytes: [0; 4],
        }
    } else {
        unsafe { *ps }
    };
    let mut s: *const c_char = unsafe { *src };
    let mut count: usize = 0;

    loop {
        if !dst.is_null() && count >= n {
            if !ps.is_null() {
                unsafe { *ps = state };
            }
            unsafe { *src = s };
            return count;
        }

        let mut wc: wchar_t = 0;
        let res: usize = unsafe { mbrtowc_core(&mut wc, s, 4, &mut state) };
        if res == SIZE_ERR || res == SIZE_INCOMPLETE {
            set_errno(EILSEQ);
            return SIZE_ERR;
        }
        if wc == 0 {
            // Per POSIX, the pointer object pointed to by `src` is updated only when `dst` is
            // non-null. When `dst` is null the call merely measures the converted length and must
            // leave `*src` untouched.
            if !dst.is_null() {
                unsafe { *dst.add(count) = 0 };
                unsafe { *src = core::ptr::null() };
            }
            if !ps.is_null() {
                unsafe { *ps = state };
            }
            return count;
        }
        if !dst.is_null() {
            unsafe { *dst.add(count) = wc };
        }
        count += 1;
        s = unsafe { s.add(res) };
    }
}

/// Restartable conversion of a wide-character string to a multibyte string.
///
/// # Safety
///
/// `src` must point to a valid pointer to a null-terminated wide string. `dst` (when non-null)
/// must have room for `n` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcsrtombs(
    dst: *mut c_char,
    src: *mut *const wchar_t,
    n: usize,
    _ps: *mut mbstate_t,
) -> usize {
    let mut w: *const wchar_t = unsafe { *src };
    let mut count: usize = 0;

    loop {
        let wc: wchar_t = unsafe { *w };
        if wc == 0 {
            // Per POSIX, the pointer object pointed to by `src` is updated only when `dst` is
            // non-null. When `dst` is null the call merely measures the converted length and must
            // leave `*src` untouched.
            if !dst.is_null() {
                if count < n {
                    // There is room for the terminating null byte: store it and signal that
                    // conversion stopped at the terminator by nulling `*src`.
                    unsafe { *dst.add(count) = 0 };
                    unsafe { *src = core::ptr::null() };
                } else {
                    // No room remains for the terminating null byte, so conversion stops early due
                    // to the length limit. `*src` is left pointing at the terminator.
                    unsafe { *src = w };
                }
            }
            return count;
        }

        let cp: u32 = cp_of(wc);
        if cp > 0xff {
            if !dst.is_null() {
                unsafe { *src = w };
            }
            set_errno(EILSEQ);
            return SIZE_ERR;
        }
        if dst.is_null() {
            count += 1;
        } else {
            if count >= n {
                unsafe { *src = w };
                return count;
            }
            unsafe { *dst.add(count) = cchar_of(u8::try_from(cp).unwrap_or(0)) };
            count += 1;
        }
        w = unsafe { w.add(1) };
    }
}

/// Restartable conversion of a multibyte string to a wide-character string, reading at most `nmc`
/// bytes from the source.
///
/// Behaves like [`mbsrtowcs`] except that no more than `nmc` bytes are read from `*src`. As with
/// [`mbsrtowcs`], conversion also stops once the terminating null byte is reached or, when `dst` is
/// non-null, once `len` wide characters have been produced.
///
/// # Safety
///
/// `src` must point to a valid pointer to a multibyte string with at least `nmc` readable bytes.
/// `dst` (when non-null) must have room for `len` wide characters.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mbsnrtowcs(
    dst: *mut wchar_t,
    src: *mut *const c_char,
    nmc: usize,
    len: usize,
    ps: *mut mbstate_t,
) -> usize {
    let mut state: mbstate_t = if ps.is_null() {
        mbstate_t {
            count: 0,
            bytes: [0; 4],
        }
    } else {
        unsafe { *ps }
    };
    let mut s: *const c_char = unsafe { *src };
    let mut remaining: usize = nmc;
    let mut count: usize = 0;

    loop {
        // Stop once the destination is full.
        if !dst.is_null() && count >= len {
            if !ps.is_null() {
                unsafe { *ps = state };
            }
            unsafe { *src = s };
            return count;
        }

        // Stop once the source byte budget is exhausted. Per POSIX, `*src` is updated only when
        // `dst` is non-null.
        if remaining == 0 {
            if !ps.is_null() {
                unsafe { *ps = state };
            }
            if !dst.is_null() {
                unsafe { *src = s };
            }
            return count;
        }

        let mut wc: wchar_t = 0;
        let res: usize = unsafe { mbrtowc_core(&mut wc, s, remaining, &mut state) };
        if res == SIZE_ERR || res == SIZE_INCOMPLETE {
            // Per POSIX, when `dst` is non-null the pointer object pointed to by `src` is updated
            // to point at the byte that triggered the encoding error, so the caller can restart or
            // diagnose. When `dst` is null the call merely measures and leaves `*src` untouched.
            if !dst.is_null() {
                unsafe { *src = s };
            }
            set_errno(EILSEQ);
            return SIZE_ERR;
        }
        if wc == 0 {
            // The terminating null was converted. Per POSIX, `*src` is nulled only when `dst` is
            // non-null.
            if !dst.is_null() {
                unsafe { *dst.add(count) = 0 };
                unsafe { *src = core::ptr::null() };
            }
            if !ps.is_null() {
                unsafe { *ps = state };
            }
            return count;
        }
        if !dst.is_null() {
            unsafe { *dst.add(count) = wc };
        }
        count += 1;
        s = unsafe { s.add(res) };
        remaining -= res;
    }
}

/// Restartable conversion of a wide-character string to a multibyte string, reading at most `nwc`
/// wide characters from the source.
///
/// Behaves like [`wcsrtombs`] except that no more than `nwc` wide characters are read from `*src`.
/// As with [`wcsrtombs`], conversion also stops once the terminating null is reached or, when `dst`
/// is non-null, once `len` bytes have been produced.
///
/// # Safety
///
/// `src` must point to a valid pointer to a wide string with at least `nwc` readable wide
/// characters. `dst` (when non-null) must have room for `len` bytes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcsnrtombs(
    dst: *mut c_char,
    src: *mut *const wchar_t,
    nwc: usize,
    len: usize,
    _ps: *mut mbstate_t,
) -> usize {
    let mut w: *const wchar_t = unsafe { *src };
    let mut remaining: usize = nwc;
    let mut count: usize = 0;

    loop {
        // Stop once the wide-character budget is exhausted. Per POSIX, `*src` is updated only when
        // `dst` is non-null.
        if remaining == 0 {
            if !dst.is_null() {
                unsafe { *src = w };
            }
            return count;
        }

        let wc: wchar_t = unsafe { *w };
        if wc == 0 {
            // Per POSIX, the pointer object pointed to by `src` is updated only when `dst` is
            // non-null.
            if !dst.is_null() {
                if count < len {
                    // There is room for the terminating null byte: store it and signal that
                    // conversion stopped at the terminator by nulling `*src`.
                    unsafe { *dst.add(count) = 0 };
                    unsafe { *src = core::ptr::null() };
                } else {
                    // No room remains for the terminating null byte, so conversion stops early due
                    // to the length limit. `*src` is left pointing at the terminator.
                    unsafe { *src = w };
                }
            }
            return count;
        }

        let cp: u32 = cp_of(wc);
        if cp > 0xff {
            if !dst.is_null() {
                unsafe { *src = w };
            }
            set_errno(EILSEQ);
            return SIZE_ERR;
        }
        if dst.is_null() {
            count += 1;
        } else {
            if count >= len {
                unsafe { *src = w };
                return count;
            }
            unsafe { *dst.add(count) = cchar_of(u8::try_from(cp).unwrap_or(0)) };
            count += 1;
        }
        w = unsafe { w.add(1) };
        remaining -= 1;
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        cchar_of,
        mbrlen,
        mbrtowc,
        mbsinit,
        mbsnrtowcs,
        mbsrtowcs,
        wcrtomb,
        wcsnrtombs,
        wcsrtombs,
        SIZE_INCOMPLETE,
    };
    use crate::{
        mbstate::mbstate_t,
        wchar_t::wchar_t,
    };

    fn as_chars(bytes: &[u8]) -> ::std::vec::Vec<::sysapi::ffi::c_char> {
        bytes.iter().map(|&byte| cchar_of(byte)).collect()
    }

    #[test]
    fn test_mbsinit_null_is_initial() {
        // A null state pointer designates the initial conversion state.
        assert_eq!(unsafe { mbsinit(core::ptr::null()) }, 1);
    }

    #[test]
    fn test_mbsinit_zero_count_is_initial() {
        let st: mbstate_t = mbstate_t {
            count: 0,
            bytes: [0; 4],
        };
        assert_eq!(unsafe { mbsinit(&st) }, 1);
    }

    #[test]
    fn test_mbsinit_pending_is_not_initial() {
        // A non-zero pending-byte count means the state is mid-sequence.
        let st: mbstate_t = mbstate_t {
            count: 2,
            bytes: [0; 4],
        };
        assert_eq!(unsafe { mbsinit(&st) }, 0);
    }

    #[test]
    fn test_mbrtowc_null_source_resets_state() {
        let mut st: mbstate_t = mbstate_t {
            count: 1,
            bytes: [0; 4],
        };
        let ret: usize = unsafe { mbrtowc(core::ptr::null_mut(), core::ptr::null(), 0, &mut st) };
        assert_eq!(ret, 0);
        assert_eq!(st.count, 0);
        assert_eq!(unsafe { mbsinit(&st) }, 1);
    }

    #[test]
    fn test_mbrtowc_accepts_high_byte_in_posix_locale() {
        let mut st: mbstate_t = mbstate_t {
            count: 0,
            bytes: [0; 4],
        };
        let mut wc: wchar_t = 0;
        let input: [::sysapi::ffi::c_char; 1] = [cchar_of(0xff)];
        let ret: usize = unsafe { mbrtowc(&mut wc, input.as_ptr(), 1, &mut st) };
        assert_eq!(ret, 1);
        assert_eq!(wc, 0xff);
        assert_eq!(unsafe { mbsinit(&st) }, 1);
    }

    #[test]
    fn test_mbrtowc_empty_input_is_incomplete() {
        let mut st: mbstate_t = mbstate_t {
            count: 0,
            bytes: [0; 4],
        };
        let input: [::sysapi::ffi::c_char; 1] = [cchar_of(0x61)];
        let ret: usize = unsafe { mbrtowc(core::ptr::null_mut(), input.as_ptr(), 0, &mut st) };
        assert_eq!(ret, SIZE_INCOMPLETE);
        assert_eq!(unsafe { mbsinit(&st) }, 1);
    }

    #[test]
    fn test_mbrtowc_nul_character_returns_zero() {
        let mut st: mbstate_t = mbstate_t {
            count: 0,
            bytes: [0; 4],
        };
        let input: [::sysapi::ffi::c_char; 1] = [0];
        let mut wc: wchar_t = -1;
        let ret: usize = unsafe { mbrtowc(&mut wc, input.as_ptr(), 1, &mut st) };
        assert_eq!(ret, 0);
        assert_eq!(wc, 0);
        assert_eq!(unsafe { mbsinit(&st) }, 1);
    }

    #[test]
    fn test_mbrlen_reports_complete_character_length() {
        let input = as_chars(&[0xff]);
        let mut st: mbstate_t = mbstate_t {
            count: 0,
            bytes: [0; 4],
        };
        let ret: usize = unsafe { mbrlen(input.as_ptr(), input.len(), &mut st) };
        assert_eq!(ret, 1);
        assert_eq!(unsafe { mbsinit(&st) }, 1);
    }

    #[test]
    fn test_wcrtomb_encodes_nul_character() {
        let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
        let ret: usize = unsafe { wcrtomb(output.as_mut_ptr(), 0, core::ptr::null_mut()) };
        assert_eq!(ret, 1);
        assert_eq!(output[0], 0);
    }

    #[test]
    fn test_mbsrtowcs_updates_source_after_partial_output() {
        let input = as_chars(b"ab\0");
        let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
        let mut output: [wchar_t; 1] = [0];
        let ret: usize = unsafe {
            mbsrtowcs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 1);
        assert_eq!(output, [0x61]);
        assert_eq!(src, unsafe { input.as_ptr().add(1) });
    }

    #[test]
    fn test_mbsrtowcs_nulls_source_after_terminator() {
        let input = as_chars(b"a\0");
        let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
        let mut output: [wchar_t; 2] = [-1; 2];
        let ret: usize = unsafe {
            mbsrtowcs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 1);
        assert_eq!(output, [0x61, 0]);
        assert!(src.is_null());
    }

    #[test]
    fn test_wcsrtombs_updates_source_after_partial_output() {
        let input: [wchar_t; 3] = [0x61, 0xff, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let mut output: [::sysapi::ffi::c_char; 1] = [0];
        let ret: usize = unsafe {
            wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 1);
        assert_eq!(output[0], cchar_of(0x61));
        assert_eq!(src, unsafe { input.as_ptr().add(1) });
    }

    #[test]
    fn test_wcsrtombs_updates_source_on_encoding_error_after_prefix() {
        let input: [wchar_t; 3] = [0x61, 0x100, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let mut output: [::sysapi::ffi::c_char; 2] = [-1; 2];
        let ret: usize = unsafe {
            wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, usize::MAX);
        assert_eq!(output[0], cchar_of(0x61));
        assert_eq!(src, unsafe { input.as_ptr().add(1) });
    }

    #[test]
    fn test_wcsrtombs_nulls_source_after_terminator() {
        let input: [wchar_t; 2] = [0x61, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let mut output: [::sysapi::ffi::c_char; 2] = [-1; 2];
        let ret: usize = unsafe {
            wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 1);
        assert_eq!(output[0], cchar_of(0x61));
        assert_eq!(output[1], 0);
        assert!(src.is_null());
    }

    #[test]
    fn test_mbsrtowcs_null_dst_leaves_source_unchanged() {
        // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched.
        let input = as_chars(b"ab\0");
        let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
        let ret: usize =
            unsafe { mbsrtowcs(core::ptr::null_mut(), &mut src, 0, core::ptr::null_mut()) };
        assert_eq!(ret, 2);
        assert_eq!(src, input.as_ptr());
    }

    #[test]
    fn test_wcsrtombs_null_dst_leaves_source_unchanged() {
        // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched.
        let input: [wchar_t; 3] = [0x61, 0x62, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let ret: usize =
            unsafe { wcsrtombs(core::ptr::null_mut(), &mut src, 0, core::ptr::null_mut()) };
        assert_eq!(ret, 2);
        assert_eq!(src, input.as_ptr());
    }

    #[test]
    fn test_wcsrtombs_no_room_for_terminator_keeps_source() {
        // When the buffer is exactly filled by the content, there is no room for the terminating
        // null byte, so conversion stops early due to the length limit and `*src` is left pointing
        // at the terminator rather than being nulled.
        let input: [wchar_t; 3] = [0x61, 0x62, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let mut output: [::sysapi::ffi::c_char; 2] = [-1; 2];
        let ret: usize = unsafe {
            wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 2);
        assert_eq!(output[0], cchar_of(0x61));
        assert_eq!(output[1], cchar_of(0x62));
        assert_eq!(src, unsafe { input.as_ptr().add(2) });
    }

    #[test]
    fn test_mbsnrtowcs_stops_at_byte_budget() {
        // Only the first two of the three available bytes may be read, so conversion stops with
        // `*src` pointing just past the last converted byte and no null is written.
        let input = as_chars(b"abc\0");
        let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
        let mut output: [wchar_t; 4] = [-1; 4];
        let ret: usize = unsafe {
            mbsnrtowcs(output.as_mut_ptr(), &mut src, 2, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 2);
        assert_eq!(output[0], 0x61);
        assert_eq!(output[1], 0x62);
        assert_eq!(src, unsafe { input.as_ptr().add(2) });
    }

    #[test]
    fn test_mbsnrtowcs_nulls_source_after_terminator_within_budget() {
        // The terminator lies within the byte budget, so it is converted and `*src` is nulled.
        let input = as_chars(b"ab\0");
        let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
        let mut output: [wchar_t; 4] = [-1; 4];
        let ret: usize = unsafe {
            mbsnrtowcs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 2);
        assert_eq!(output[0], 0x61);
        assert_eq!(output[1], 0x62);
        assert_eq!(output[2], 0);
        assert!(src.is_null());
    }

    #[test]
    fn test_mbsnrtowcs_stops_when_destination_full() {
        // The destination holds a single wide character, so conversion stops there even though more
        // input and byte budget remain.
        let input = as_chars(b"abc\0");
        let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
        let mut output: [wchar_t; 1] = [0];
        let ret: usize = unsafe {
            mbsnrtowcs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 1);
        assert_eq!(output[0], 0x61);
        assert_eq!(src, unsafe { input.as_ptr().add(1) });
    }

    #[test]
    fn test_mbsnrtowcs_null_dst_leaves_source_unchanged() {
        // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched, while the byte
        // budget still bounds the reported length.
        let input = as_chars(b"abcd\0");
        let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
        let ret: usize =
            unsafe { mbsnrtowcs(core::ptr::null_mut(), &mut src, 3, 0, core::ptr::null_mut()) };
        assert_eq!(ret, 3);
        assert_eq!(src, input.as_ptr());
    }

    #[test]
    fn test_wcsnrtombs_stops_at_wide_char_budget() {
        // Only the first two of the three available wide characters may be read, so conversion stops
        // with `*src` pointing just past the last converted wide character.
        let input: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
        let ret: usize = unsafe {
            wcsnrtombs(output.as_mut_ptr(), &mut src, 2, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 2);
        assert_eq!(output[0], cchar_of(0x61));
        assert_eq!(output[1], cchar_of(0x62));
        assert_eq!(src, unsafe { input.as_ptr().add(2) });
    }

    #[test]
    fn test_wcsnrtombs_nulls_source_after_terminator_within_budget() {
        // The terminator lies within the wide-character budget, so it is converted and `*src` is
        // nulled.
        let input: [wchar_t; 3] = [0x61, 0x62, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
        let ret: usize = unsafe {
            wcsnrtombs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, 2);
        assert_eq!(output[0], cchar_of(0x61));
        assert_eq!(output[1], cchar_of(0x62));
        assert_eq!(output[2], 0);
        assert!(src.is_null());
    }

    #[test]
    fn test_wcsnrtombs_reports_error_on_unencodable_wide_char() {
        // A wide character outside the single-byte range aborts conversion with `EILSEQ` and leaves
        // `*src` pointing at the offending character.
        let input: [wchar_t; 3] = [0x61, 0x100, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
        let ret: usize = unsafe {
            wcsnrtombs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
        };
        assert_eq!(ret, usize::MAX);
        assert_eq!(output[0], cchar_of(0x61));
        assert_eq!(src, unsafe { input.as_ptr().add(1) });
    }

    #[test]
    fn test_wcsnrtombs_null_dst_leaves_source_unchanged() {
        // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched, while the
        // wide-character budget still bounds the reported length.
        let input: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
        let mut src: *const wchar_t = input.as_ptr();
        let ret: usize =
            unsafe { wcsnrtombs(core::ptr::null_mut(), &mut src, 2, 0, core::ptr::null_mut()) };
        assert_eq!(ret, 2);
        assert_eq!(src, input.as_ptr());
    }
}
