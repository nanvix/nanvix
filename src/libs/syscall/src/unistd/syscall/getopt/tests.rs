// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
fn optind_zero_restarts_option_processing() {
    let args = Argv::new(&["prog", "-a", "-b"]);
    let optstring = CString::new("ab").expect("no nul");
    let mut state = GetoptState::default();
    // First pass consumes both options and reaches the end.
    assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
    assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'b'));
    assert_eq!(unsafe { step(&mut state, &args, &optstring) }, -1);
    assert_eq!(state.optind, 3);
    // Setting optind to 0 (GNU reset convention) restarts parsing from the first argument.
    state.optind = 0;
    assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
    assert_eq!(state.optind, 2);
}

#[test]
fn optind_zero_clears_clustered_cursor() {
    let args = Argv::new(&["prog", "-ab"]);
    let optstring = CString::new("ab").expect("no nul");
    let mut state = GetoptState::default();
    // Consume only the first character of the cluster, leaving the cursor mid-element.
    assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
    // Resetting before the cluster is exhausted must restart at argv[1], not resume at 'b'.
    state.optind = 0;
    assert_eq!(unsafe { step(&mut state, &args, &optstring) }, opt(b'a'));
    assert_eq!(state.optind, 1);
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
