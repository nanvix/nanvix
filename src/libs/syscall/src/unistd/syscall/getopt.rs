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
// Structures
//==================================================================================================

///
/// # Description
///
/// Mutable parser state threaded through successive [`getopt`] calls.
///
/// The C ABI bindings mirror the [`optarg`](GetoptState::optarg), [`optind`](GetoptState::optind),
/// and [`optopt`](GetoptState::optopt) fields onto the global `optarg`/`optind`/`optopt` symbols
/// (see `crate::unistd::bindings::getopt`). Keeping the parser state in a dedicated structure makes
/// the implementation host-testable without relying on mutable global statics.
///
pub struct GetoptState {
    /// Argument of the current option, when it takes one (mirrors the global `optarg`).
    pub optarg: *mut c_char,
    /// Index of the next element of `argv` to process (mirrors the global `optind`).
    pub optind: c_int,
    /// The option character that caused the most recent error (mirrors the global `optopt`).
    pub optopt: c_int,
    /// Cursor into the current clustered short-option element (internal parser state).
    pub nextchar: *const c_char,
}

impl Default for GetoptState {
    fn default() -> Self {
        Self {
            optarg: ::core::ptr::null_mut(),
            // POSIX requires `optind` to be initialized to 1 by the system.
            optind: 1,
            optopt: 0,
            nextchar: ::core::ptr::null(),
        }
    }
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns a pointer to the first occurrence of `c` in the optstring `s`, or null.
///
/// # Safety
///
/// `s` must point to a valid, null-terminated C string.
unsafe fn find_opt(s: *const c_char, c: c_char) -> *const c_char {
    let mut p: *const c_char = s;
    while unsafe { *p } != 0 {
        if unsafe { *p } == c {
            return p;
        }
        p = unsafe { p.add(1) };
    }
    ::core::ptr::null()
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Parses command-line options from `argv` according to the option specification `optstring`,
/// updating `state` in place. This is the back-end implementation shared by the C ABI binding
/// `getopt()`; it deliberately avoids global state so that it can be unit-tested on the host.
///
/// A leading `<plus-sign>` (`'+'`) in `optstring` is accepted and ignored, as mandated by POSIX:
/// Nanvix already implements the conforming (non-permuting) scanning behavior, so the `'+'` has no
/// effect other than being skipped before option matching.
///
/// # Parameters
///
/// - `state`: Parser state carried across successive calls.
/// - `argc`: Argument count.
/// - `argv`: Argument vector.
/// - `optstring`: Recognized option characters; a character followed by `:` takes an argument. An
///   optional leading `:` (after an optional leading `+`) selects the missing-argument variant
///   that returns `':'`.
///
/// # Returns
///
/// The next option character, `'?'` for an unknown option or a missing argument when `optstring`
/// does not begin with `:`, `':'` for a missing argument when `optstring` begins with `:`, or `-1`
/// when option parsing is complete.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. `argv` must contain `argc` valid,
/// null-terminated strings and `optstring` must be a valid, null-terminated string.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getopt.html>
///
pub unsafe fn getopt(
    state: &mut GetoptState,
    argc: c_int,
    argv: *const *mut c_char,
    optstring: *const c_char,
) -> c_int {
    // GNU/glibc reset convention: a caller restarts option processing by setting `optind` to 0.
    // POSIX initializes `optind` to 1 and never assigns it 0, so the value is reserved to mean
    // "reinitialize". Honor it by rewinding to the first argument and discarding any partially
    // consumed clustered short-option element, matching the behavior relied upon by programs such
    // as BusyBox that re-run option parsing in-process.
    if state.optind == 0 {
        state.optind = 1;
        state.nextchar = ::core::ptr::null();
    }

    // A leading '+' forces POSIX-conforming behavior in otherwise non-conforming environments and
    // has no other effect here; skip it so it is never treated as an option character. When both
    // '+' and ':' lead the string, POSIX requires '+' to appear first.
    let spec: *const c_char = if unsafe { *optstring } == b'+' as c_char {
        unsafe { optstring.add(1) }
    } else {
        optstring
    };

    // Start (or continue) scanning the current argv element.
    let need_new: bool = state.nextchar.is_null() || unsafe { *state.nextchar } == 0;
    if need_new {
        let idx: c_int = state.optind;
        if idx >= argc {
            return -1;
        }
        let arg: *const c_char = unsafe { *argv.offset(idx as isize) };
        // Not an option (null, does not start with '-', or is just "-").
        if arg.is_null() || unsafe { *arg } != b'-' as c_char || unsafe { *arg.add(1) } == 0 {
            return -1;
        }
        // "--" terminates option processing.
        if unsafe { *arg.add(1) } == b'-' as c_char && unsafe { *arg.add(2) } == 0 {
            state.optind += 1;
            return -1;
        }
        state.nextchar = unsafe { arg.add(1) };
    }

    // Consume one option character.
    let c: c_char = unsafe { *state.nextchar };
    state.nextchar = unsafe { state.nextchar.add(1) };

    let colon_lead: bool = unsafe { *spec } == b':' as c_char;
    let pos: *const c_char = unsafe { find_opt(spec, c) };
    if pos.is_null() || c == b':' as c_char {
        state.optopt = c as c_int;
        // Advance past this argv element when it is exhausted.
        if unsafe { *state.nextchar } == 0 {
            state.optind += 1;
            state.nextchar = ::core::ptr::null();
        }
        return b'?' as c_int;
    }

    // Does this option take an argument?
    if unsafe { *pos.add(1) } == b':' as c_char {
        if unsafe { *state.nextchar } != 0 {
            // Argument is the remainder of the current element, so `optind` is incremented by 1.
            state.optarg = state.nextchar.cast_mut();
            state.optind += 1;
            state.nextchar = ::core::ptr::null();
        } else {
            // Argument is the next element. POSIX specifies that `optind` is incremented by 2 (past
            // both the option element and its option-argument) and that a resulting value greater
            // than `argc` indicates a missing option-argument.
            state.optind += 2;
            state.nextchar = ::core::ptr::null();
            if state.optind > argc {
                // The option was the last element of `argv`, so its argument is missing.
                state.optopt = c as c_int;
                return if colon_lead {
                    b':' as c_int
                } else {
                    b'?' as c_int
                };
            }
            state.optarg = unsafe { *argv.offset((state.optind - 1) as isize) };
        }
    } else {
        // No argument: advance past the element when exhausted.
        if unsafe { *state.nextchar } == 0 {
            state.optind += 1;
            state.nextchar = ::core::ptr::null();
        }
    }

    c as c_int
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
