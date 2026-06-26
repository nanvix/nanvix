// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::unistd::syscall::{
    self,
    GetoptState,
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};
#[cfg(feature = "syscall")]
use ::sysapi::unistd::STDERR_FILENO;

//==================================================================================================
// Structures
//==================================================================================================

/// Diagnostic kind reported for an unsuccessful `getopt()` step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GetoptDiagnostic {
    /// The current option character was not recognized.
    IllegalOption,
    /// The current option requires an option-argument, but none was provided.
    MissingArgument,
}

//==================================================================================================
// Global State
//==================================================================================================

// The `no_mangle` attribute is gated off under the `std` feature so that host unit tests do not
// export symbols that collide with the system C library's `getopt` machinery. The `no_mangle`
// attribute also suppresses `non_upper_case_globals`, so the lint allowance is only required in the
// `std` (test) configuration.

/// Argument of the current option, when it takes one.
#[allow(non_upper_case_globals)]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static mut optarg: *mut c_char = ::core::ptr::null_mut();

/// Index of the next element of `argv` to process.
#[allow(non_upper_case_globals)]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static mut optind: c_int = 1;

/// Controls whether `getopt` prints error messages.
#[allow(non_upper_case_globals)]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static mut opterr: c_int = 1;

/// The option character that caused an error.
#[allow(non_upper_case_globals)]
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub static mut optopt: c_int = 0;

/// Cursor into the current clustered short-option element (internal parser state).
static mut NEXTCHAR: *const c_char = ::core::ptr::null();

/// Discards the internal short-option cursor shared with [`getopt`].
///
/// This supports the GNU `optind == 0` reset convention: a caller that restarts option processing
/// by zeroing `optind` must also drop any partially consumed clustered short-option element so the
/// next call begins at a fresh argument. The companion [`getopt_long`](super::getopt_long::getopt_long)
/// entry point invokes this when it observes the reset request.
///
/// # Safety
///
/// The global `getopt` state must not be accessed concurrently from another thread.
pub(crate) unsafe fn reset_short_option_cursor() {
    unsafe { NEXTCHAR = ::core::ptr::null() };
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `optstring` after its optional leading <plus-sign>.
///
/// # Safety
///
/// `optstring` must point to a valid, null-terminated C string.
unsafe fn effective_optstring(optstring: *const c_char) -> *const c_char {
    if unsafe { *optstring } == b'+' as c_char {
        unsafe { optstring.add(1) }
    } else {
        optstring
    }
}

/// Returns `true` if `option` is specified in `optstring` and takes an argument.
///
/// # Safety
///
/// `optstring` must point to a valid, null-terminated C string without the optional leading
/// <plus-sign>.
unsafe fn option_requires_argument(optstring: *const c_char, option: c_int) -> bool {
    if option == b':' as c_int {
        return false;
    }

    let option: c_char = option as c_char;
    let mut p: *const c_char = optstring;
    while unsafe { *p } != 0 {
        if unsafe { *p } == option {
            return unsafe { *p.add(1) } == b':' as c_char;
        }
        p = unsafe { p.add(1) };
    }

    false
}

/// Returns the diagnostic that should be printed for a `getopt()` result, if any.
///
/// # Safety
///
/// `optstring` must point to a valid, null-terminated C string.
unsafe fn diagnostic_kind(
    rc: c_int,
    option: c_int,
    optstring: *const c_char,
    opterr_value: c_int,
) -> Option<GetoptDiagnostic> {
    if opterr_value == 0 || rc != b'?' as c_int {
        return None;
    }

    let optstring: *const c_char = unsafe { effective_optstring(optstring) };
    if unsafe { *optstring } == b':' as c_char {
        return None;
    }

    if unsafe { option_requires_argument(optstring, option) } {
        Some(GetoptDiagnostic::MissingArgument)
    } else {
        Some(GetoptDiagnostic::IllegalOption)
    }
}

/// Emits a diagnostic message when the current build can write to standard error.
fn emit_diagnostic(argv: *const *mut c_char, kind: GetoptDiagnostic, option: c_int) {
    #[cfg(any(feature = "syscall", feature = "std"))]
    unsafe {
        write_diagnostic(argv, kind, option);
    }

    #[cfg(not(any(feature = "syscall", feature = "std")))]
    let _ = (argv, kind, option);
}

/// Writes a POSIX-style `getopt()` diagnostic to standard error.
///
/// # Safety
///
/// `argv` must either be null or point to an argument vector whose first element is null or a
/// valid, null-terminated C string.
#[cfg(any(feature = "syscall", feature = "std"))]
unsafe fn write_diagnostic(argv: *const *mut c_char, kind: GetoptDiagnostic, option: c_int) {
    unsafe { write_program_name(argv) };
    match kind {
        GetoptDiagnostic::IllegalOption => write_stderr(b": illegal option -- "),
        GetoptDiagnostic::MissingArgument => write_stderr(b": option requires an argument -- "),
    }
    write_stderr(&[option as u8]);
    write_stderr(b"\n");
}

/// Writes `argv[0]`'s basename to standard error.
///
/// # Safety
///
/// `argv` must either be null or point to an argument vector whose first element is null or a
/// valid, null-terminated C string.
#[cfg(any(feature = "syscall", feature = "std"))]
unsafe fn write_program_name(argv: *const *mut c_char) {
    if argv.is_null() {
        write_stderr(b"getopt");
        return;
    }

    let arg0: *const c_char = unsafe { *argv };
    if arg0.is_null() {
        write_stderr(b"getopt");
        return;
    }

    let mut p: *const u8 = arg0.cast::<u8>();
    let mut basename: *const u8 = p;
    let mut basename_len: usize = 0;
    while unsafe { *p } != 0 {
        if unsafe { *p } == b'/' {
            basename = unsafe { p.add(1) };
            basename_len = 0;
        } else {
            basename_len += 1;
        }
        p = unsafe { p.add(1) };
    }

    if basename_len == 0 {
        write_stderr(b"getopt");
        return;
    }

    let program: &[u8] = unsafe { ::core::slice::from_raw_parts(basename, basename_len) };
    write_stderr(program);
}

/// Writes bytes to standard error, ignoring failures as POSIX requires `getopt()` to still return.
#[cfg(feature = "syscall")]
fn write_stderr(mut bytes: &[u8]) {
    while !bytes.is_empty() {
        match syscall::write(STDERR_FILENO, bytes) {
            Ok(bytes_written) => {
                let bytes_written: usize = bytes_written as usize;
                if bytes_written == 0 || bytes_written > bytes.len() {
                    break;
                }
                bytes = &bytes[bytes_written..];
            },
            Err(_) => break,
        }
    }
}

/// Writes bytes to standard error in host-test builds.
#[cfg(all(feature = "std", not(feature = "syscall")))]
fn write_stderr(bytes: &[u8]) {
    use ::std::io::Write;

    let _ = ::std::io::stderr().write_all(bytes);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Parses command-line options from `argv` according to the option specification `optstring`. This
/// is the C ABI entry point; the parsing logic lives in [`crate::unistd::syscall::getopt`]. The
/// global `optarg`, `optind`, and `optopt` variables are mirrored into the parser state on entry
/// and written back on return.
///
/// # Parameters
///
/// - `argc`: Argument count.
/// - `argv`: Argument vector.
/// - `optstring`: Recognized option characters; a character followed by `:` takes an argument.
///
/// # Returns
///
/// The next option character, `'?'` for an unknown option, `':'` for a missing argument when
/// `optstring` begins with `:`, or `-1` when option parsing is complete.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and modifies global state. `argv`
/// must contain `argc` valid, null-terminated strings and `optstring` must be a valid,
/// null-terminated string. The global `getopt` state must not be accessed concurrently from
/// another thread.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getopt.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getopt(
    argc: c_int,
    argv: *const *mut c_char,
    optstring: *const c_char,
) -> c_int {
    // Load the C-visible globals (and the internal cursor) into the parser state.
    let mut state: GetoptState = GetoptState {
        optarg: unsafe { optarg },
        optind: unsafe { optind },
        optopt: unsafe { optopt },
        nextchar: unsafe { NEXTCHAR },
    };

    // Run the parser.
    let rc: c_int = unsafe { syscall::getopt(&mut state, argc, argv, optstring) };

    if let Some(kind) = unsafe { diagnostic_kind(rc, state.optopt, optstring, opterr) } {
        emit_diagnostic(argv, kind, state.optopt);
    }

    // Write the updated state back to the C-visible globals.
    unsafe {
        optarg = state.optarg;
        optind = state.optind;
        optopt = state.optopt;
        NEXTCHAR = state.nextchar;
    }

    rc
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        diagnostic_kind,
        GetoptDiagnostic,
    };
    use ::std::ffi::CString;
    use ::sysapi::ffi::{
        c_char,
        c_int,
    };

    /// Returns the option character `c` as the [`c_int`] value `getopt` reports.
    fn opt(c: u8) -> c_int {
        c_int::from(c)
    }

    /// Returns the diagnostic decision for one synthetic `getopt()` result.
    fn diagnostic(
        rc: u8,
        option: u8,
        optstring: &str,
        opterr_value: c_int,
    ) -> Option<GetoptDiagnostic> {
        let optstring: CString = CString::new(optstring).expect("no interior nul");
        unsafe {
            diagnostic_kind(opt(rc), opt(option), optstring.as_ptr().cast::<c_char>(), opterr_value)
        }
    }

    #[test]
    fn diagnostic_reports_unknown_option() {
        assert_eq!(diagnostic(b'?', b'x', "a", 1), Some(GetoptDiagnostic::IllegalOption));
    }

    #[test]
    fn diagnostic_reports_missing_argument() {
        assert_eq!(diagnostic(b'?', b'a', "a:", 1), Some(GetoptDiagnostic::MissingArgument));
    }

    #[test]
    fn diagnostic_treats_colon_option_as_illegal_option() {
        assert_eq!(diagnostic(b'?', b':', "a::", 1), Some(GetoptDiagnostic::IllegalOption));
    }

    #[test]
    fn diagnostic_is_suppressed_when_opterr_is_zero() {
        assert_eq!(diagnostic(b'?', b'x', "a", 0), None);
    }

    #[test]
    fn diagnostic_is_suppressed_by_leading_colon() {
        assert_eq!(diagnostic(b'?', b'x', ":a", 1), None);
    }

    #[test]
    fn diagnostic_is_suppressed_by_leading_plus_then_colon() {
        assert_eq!(diagnostic(b'?', b'a', "+:a:", 1), None);
    }

    #[test]
    fn diagnostic_is_not_reported_for_successful_option() {
        assert_eq!(diagnostic(b'a', b'a', "a", 1), None);
    }
}
