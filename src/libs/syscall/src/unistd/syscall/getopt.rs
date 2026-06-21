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
mod tests {
    use super::{
        getopt,
        GetoptState,
    };
    use ::std::{
        ffi::{
            CStr,
            CString,
        },
        string::String,
        vec::Vec,
    };
    use ::sysapi::ffi::{
        c_char,
        c_int,
    };

    /// Owns the backing storage for a synthetic `argv` vector used in tests.
    struct Argv {
        // Keeps the C strings alive for as long as the pointer vector is in use.
        _storage: Vec<CString>,
        ptrs: Vec<*mut c_char>,
    }

    impl Argv {
        /// Builds an `argv` from the given argument strings.
        fn new(args: &[&str]) -> Self {
            let storage: Vec<CString> = args
                .iter()
                .map(|s| CString::new(*s).expect("no interior nul"))
                .collect();
            let ptrs: Vec<*mut c_char> = storage.iter().map(|c| c.as_ptr().cast_mut()).collect();
            Self {
                _storage: storage,
                ptrs,
            }
        }

        /// Returns the argument count.
        fn argc(&self) -> c_int {
            self.ptrs.len() as c_int
        }

        /// Returns the argument vector pointer.
        fn argv(&self) -> *const *mut c_char {
            self.ptrs.as_ptr()
        }
    }

    /// Returns the option character `c` as the [`c_int`] value `getopt` reports.
    fn opt(c: u8) -> c_int {
        c_int::from(c)
    }

    /// Returns the current `optarg` as an owned string, or [`None`] when it is null.
    fn optarg_string(state: &GetoptState) -> Option<String> {
        if state.optarg.is_null() {
            return None;
        }
        // SAFETY: when non-null, `optarg` points into a valid null-terminated argv string.
        Some(
            unsafe { CStr::from_ptr(state.optarg) }
                .to_string_lossy()
                .into_owned(),
        )
    }

    /// Runs one `getopt` step against `args`/`optstring` using `state`.
    unsafe fn step(state: &mut GetoptState, args: &Argv, optstring: &CStr) -> c_int {
        unsafe { getopt(state, args.argc(), args.argv(), optstring.as_ptr()) }
    }

    #[test]
    fn program_name_only_returns_minus_one() {
        let args = Argv::new(&["prog"]);
        let optstring = CString::new("a").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        // optind must be left unchanged when there is nothing to parse.
        assert_eq!(state.optind, 1);
    }

    #[test]
    fn single_flag() {
        let args = Argv::new(&["prog", "-a"]);
        let optstring = CString::new("a").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        assert_eq!(state.optind, 2);
    }

    #[test]
    fn clustered_flags() {
        let args = Argv::new(&["prog", "-abc"]);
        let optstring = CString::new("abc").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'b'));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'c'));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        assert_eq!(state.optind, 2);
    }

    #[test]
    fn unknown_option_returns_question_mark() {
        let args = Argv::new(&["prog", "-x"]);
        let optstring = CString::new("a").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'?'));
        assert_eq!(state.optopt, opt(b'x'));
    }

    #[test]
    fn option_with_attached_argument() {
        let args = Argv::new(&["prog", "-avalue"]);
        let optstring = CString::new("a:").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
        assert_eq!(optarg_string(&state).as_deref(), Some("value"));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
    }

    #[test]
    fn option_with_separate_argument_advances_optind_by_two() {
        let args = Argv::new(&["prog", "-a", "value"]);
        let optstring = CString::new("a:").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
        assert_eq!(optarg_string(&state).as_deref(), Some("value"));
        // optind moves from 1 past both the option and its argument.
        assert_eq!(state.optind, 3);
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
    }

    #[test]
    fn missing_argument_without_leading_colon_returns_question_mark() {
        let args = Argv::new(&["prog", "-a"]);
        let optstring = CString::new("a:").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'?'));
        assert_eq!(state.optopt, opt(b'a'));
        // POSIX increments optind by 2 (to argc+1) when the missing argument was the last element.
        assert_eq!(state.optind, args.argc() + 1);
    }

    #[test]
    fn missing_argument_with_leading_colon_returns_colon() {
        let args = Argv::new(&["prog", "-a"]);
        let optstring = CString::new(":a:").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b':'));
        assert_eq!(state.optopt, opt(b'a'));
        // POSIX increments optind by 2 (to argc+1) when the missing argument was the last element.
        assert_eq!(state.optind, args.argc() + 1);
    }

    #[test]
    fn missing_separate_argument_leaves_optind_past_argc_then_stops() {
        // POSIX: "optind shall be incremented by 2. If the resulting value of optind is greater
        // than argc, this indicates a missing option-argument." A subsequent call must therefore
        // report end-of-options without re-processing or reading past argv.
        let args = Argv::new(&["prog", "-a"]);
        let optstring = CString::new("a:").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'?'));
        assert_eq!(state.optind, args.argc() + 1);
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        assert_eq!(state.optind, args.argc() + 1);
    }

    #[test]
    fn double_dash_terminates_and_advances_optind() {
        let args = Argv::new(&["prog", "--", "-a"]);
        let optstring = CString::new("a").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        // optind is incremented past the "--" element.
        assert_eq!(state.optind, 2);
    }

    #[test]
    fn single_dash_is_not_an_option() {
        let args = Argv::new(&["prog", "-"]);
        let optstring = CString::new("a").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        // "-" is treated as a non-option operand; optind is left unchanged.
        assert_eq!(state.optind, 1);
    }

    #[test]
    fn stops_at_first_non_option_operand() {
        let args = Argv::new(&["prog", "-a", "file", "-b"]);
        let optstring = CString::new("ab").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
        // The operand "file" stops scanning; "-b" past it is not parsed.
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        assert_eq!(state.optind, 2);
    }

    #[test]
    fn colon_query_returns_question_mark() {
        let args = Argv::new(&["prog", "-:"]);
        let optstring = CString::new("a:").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'?'));
        assert_eq!(state.optopt, opt(b':'));
    }

    #[test]
    fn leading_plus_is_ignored() {
        let args = Argv::new(&["prog", "-a"]);
        let optstring = CString::new("+a").expect("no nul");
        let mut state = GetoptState::default();
        // The leading '+' must not be matched as an option and must not change behavior.
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
    }

    #[test]
    fn leading_plus_does_not_match_as_option() {
        let args = Argv::new(&["prog", "-+"]);
        let optstring = CString::new("+a").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'?'));
        assert_eq!(state.optopt, opt(b'+'));
    }

    #[test]
    fn leading_plus_then_colon_enables_colon_mode() {
        let args = Argv::new(&["prog", "-a"]);
        let optstring = CString::new("+:a:").expect("no nul");
        let mut state = GetoptState::default();
        // With '+' skipped, the leading ':' still selects the colon (missing-argument) variant.
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b':'));
        assert_eq!(state.optopt, opt(b'a'));
    }

    #[test]
    fn full_scan_mixed_options() {
        // Mirrors the canonical POSIX example optstring ":abf:o:".
        let args = Argv::new(&["prog", "-a", "-o", "arg", "path"]);
        let optstring = CString::new(":abf:o:").expect("no nul");
        let mut state = GetoptState::default();
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'o'));
        assert_eq!(optarg_string(&state).as_deref(), Some("arg"));
        assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
        // optind points at the first non-option operand ("path").
        assert_eq!(state.optind, 4);
    }
}
