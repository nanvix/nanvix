// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Shared backing logic for the `mktemp`-family functions: validating the caller's template and
//! filling its trailing `XXXXXX` placeholder with pseudo-random characters.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::sysapi::ffi::c_char;

//==================================================================================================
// Constants
//==================================================================================================

/// Number of trailing `X` characters a template must end with.
pub(crate) const SUFFIX_LEN: usize = 6;

/// Portable alphabet used to fill the random suffix.
const CHARS: &[u8; 62] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

//==================================================================================================
// Static Data
//==================================================================================================

/// Per-call counter that perturbs the seed so successive calls produce distinct names.
static COUNTER: AtomicU32 = AtomicU32::new(0);

//==================================================================================================
// Internal Functions
//==================================================================================================

///
/// # Description
///
/// Validates a `mktemp`-style template. The template must be a non-null C string of at least
/// [`SUFFIX_LEN`] characters whose final [`SUFFIX_LEN`] characters are all `'X'`.
///
/// # Returns
///
/// The length of the template (excluding the null terminator) on success, or `None` if the template
/// is null or does not end with the required `XXXXXX` placeholder.
///
/// # Safety
///
/// `tmpl`, when non-null, must point to a valid null-terminated string.
///
pub(crate) unsafe fn validate(tmpl: *const c_char) -> Option<usize> {
    if tmpl.is_null() {
        return None;
    }

    let base: *const u8 = tmpl.cast::<u8>();
    let mut len: usize = 0;
    // SAFETY: tmpl is a valid null-terminated string, so the scan stops at the null byte.
    while unsafe { *base.add(len) } != 0 {
        len += 1;
    }

    if len < SUFFIX_LEN {
        return None;
    }

    let mut i: usize = 0;
    while i < SUFFIX_LEN {
        // SAFETY: len - SUFFIX_LEN + i is within the string bounds.
        if unsafe { *base.add(len - SUFFIX_LEN + i) } != b'X' {
            return None;
        }
        i += 1;
    }

    Some(len)
}

///
/// # Description
///
/// Overwrites the final [`SUFFIX_LEN`] bytes of the template with pseudo-random characters drawn
/// from a portable alphabet. The seed combines the process identifier with a monotonically
/// increasing counter so that repeated calls (including retries after a collision) yield different
/// suffixes.
///
/// # Parameters
///
/// - `tmpl`: Pointer to the template whose suffix is to be replaced.
/// - `len`: Length of the template, as returned by [`validate`].
///
/// # Safety
///
/// `tmpl` must point to a writable buffer holding at least `len` bytes, with `len >= SUFFIX_LEN`.
///
pub(crate) unsafe fn randomize_suffix(tmpl: *mut c_char, len: usize) {
    let pid_bits: u32 = pid_seed();
    let counter: u32 = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut state: u32 = pid_bits ^ counter.wrapping_mul(2_654_435_761);

    let dst: *mut u8 = tmpl.cast::<u8>();
    let start: usize = len - SUFFIX_LEN;
    let mut i: usize = 0;
    while i < SUFFIX_LEN {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let idx: usize = ((state >> 16) % 62) as usize;
        // SAFETY: start + i < len, so the write stays within the template buffer.
        unsafe {
            *dst.add(start + i) = CHARS[idx];
        }
        i += 1;
    }
}

/// Returns the process identifier, used to perturb the random suffix seed.
///
/// In the freestanding libc this resolves to the `getpid` symbol provided by libposix startup. In a
/// hosted (unit-test) build there is no such symbol to link against, so a constant seed is used and
/// distinct names rely solely on the per-call [`COUNTER`].
#[cfg(not(feature = "std"))]
fn pid_seed() -> u32 {
    extern "C" {
        fn getpid() -> ::sysapi::sys_types::pid_t;
    }

    // SAFETY: getpid() takes no arguments and has no preconditions.
    u32::from_ne_bytes(unsafe { getpid() }.to_ne_bytes())
}

/// Hosted-build stand-in for [`pid_seed`]; see its documentation.
#[cfg(feature = "std")]
fn pid_seed() -> u32 {
    0
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        randomize_suffix,
        validate,
        SUFFIX_LEN,
    };
    use ::sysapi::ffi::c_char;

    #[test]
    fn test_validate_accepts_suffix() {
        let tmpl: &[u8] = b"foo.XXXXXX\0";
        let len: Option<usize> = unsafe { validate(tmpl.as_ptr().cast::<c_char>()) };
        assert_eq!(len, Some(10));
    }

    #[test]
    fn test_validate_rejects_short() {
        let tmpl: &[u8] = b"XXXXX\0";
        assert_eq!(unsafe { validate(tmpl.as_ptr().cast::<c_char>()) }, None);
    }

    #[test]
    fn test_validate_rejects_missing_suffix() {
        let tmpl: &[u8] = b"foobar\0";
        assert_eq!(unsafe { validate(tmpl.as_ptr().cast::<c_char>()) }, None);
    }

    #[test]
    fn test_validate_rejects_null() {
        assert_eq!(unsafe { validate(core::ptr::null()) }, None);
    }

    #[test]
    fn test_randomize_replaces_suffix() {
        let mut tmpl: [u8; 11] = *b"foo.XXXXXX\0";
        let len: usize = unsafe { validate(tmpl.as_ptr().cast::<c_char>()) }.unwrap_or(0);
        assert_eq!(len, 10);
        unsafe { randomize_suffix(tmpl.as_mut_ptr().cast::<c_char>(), len) };

        // The fixed prefix and null terminator are untouched.
        assert_eq!(&tmpl[..4], b"foo.");
        assert_eq!(tmpl[10], 0);
        // Every suffix byte is replaced with an alphanumeric character from the alphabet.
        for &b in &tmpl[len - SUFFIX_LEN..len] {
            assert!(b.is_ascii_alphanumeric(), "byte {b:#x} not in alphabet");
        }
    }
}
