// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::getopt::{
    getopt,
    optarg,
    optind,
    optopt,
    reset_short_option_cursor,
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// `has_arg` value: the long option takes no argument.
pub const NO_ARGUMENT: c_int = 0;
/// `has_arg` value: the long option requires an argument.
pub const REQUIRED_ARGUMENT: c_int = 1;
/// `has_arg` value: the long option takes an optional argument.
pub const OPTIONAL_ARGUMENT: c_int = 2;

/// The `'-'` path separator that introduces an option.
const DASH: c_char = b'-' as c_char;
/// The `'='` byte that separates a long option name from its inline argument.
const EQUALS: c_char = b'=' as c_char;
/// The `':'` leading byte of `optstring` that selects the missing-argument return.
const COLON: c_char = b':' as c_char;
/// The `'?'` return value reported for an unknown or ambiguous option.
const QUESTION: c_int = b'?' as c_int;

//==================================================================================================
// Structures
//==================================================================================================

/// Result of resolving a possibly abbreviated long option name.
enum LongOptionMatch {
    /// No long option matched the requested name.
    NoMatch,
    /// More than one long option matched the requested abbreviation.
    Ambiguous,
    /// Exactly one long option matched the requested name.
    Matched(usize),
}

/// Arguments that are common to one `getopt_long()` or `getopt_long_only()` call.
struct LongOptionContext {
    /// Argument count.
    argc: c_int,
    /// Argument vector.
    argv: *const *mut c_char,
    /// Recognized short options.
    optstring: *const c_char,
    /// Long option table.
    longopts: *const LongOption,
    /// Optional long option index output.
    longindex: *mut c_int,
}

/// A long option that has been matched and is ready to be consumed.
struct ResolvedLongOption {
    /// Matched option name inside the current argv element.
    name: *mut c_char,
    /// Inline argument separator position, when present.
    eq: Option<usize>,
    /// Matched index inside the long option table.
    matched: usize,
}

///
/// # Description
///
/// A single entry in the array of long options passed to [`getopt_long`]. Mirrors the GNU
/// `struct option` layout.
///
#[repr(C)]
pub struct LongOption {
    /// The long option name (without the leading `"--"`).
    pub name: *const c_char,
    /// One of [`NO_ARGUMENT`], [`REQUIRED_ARGUMENT`], or [`OPTIONAL_ARGUMENT`].
    pub has_arg: c_int,
    /// If non-null, `getopt_long` stores `val` here and returns `0`.
    pub flag: *mut c_int,
    /// The value to return (or store through `flag`) when this option is matched.
    pub val: c_int,
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns the length of the null-terminated C string `p`, excluding the terminator.
///
/// # Safety
///
/// `p` must point to a valid null-terminated C string.
unsafe fn c_strlen(p: *const c_char) -> usize {
    unsafe {
        let mut n: usize = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        n
    }
}

/// Returns the index of the first `'='` in `p`, or `None` if there is none.
///
/// # Safety
///
/// `p` must point to a valid null-terminated C string.
unsafe fn find_equals(p: *const c_char) -> Option<usize> {
    unsafe {
        let mut n: usize = 0;
        loop {
            let c: c_char = *p.add(n);
            if c == 0 {
                return None;
            }
            if c == EQUALS {
                return Some(n);
            }
            n += 1;
        }
    }
}

/// Returns `true` if the first `n` bytes of `a` and `b` are equal (i.e. `strncmp(a, b, n) == 0`).
///
/// # Safety
///
/// `a` and `b` must point to valid null-terminated C strings.
unsafe fn prefix_equals(a: *const c_char, b: *const c_char, n: usize) -> bool {
    unsafe {
        let mut i: usize = 0;
        while i < n {
            let ca: c_char = *a.add(i);
            let cb: c_char = *b.add(i);
            if ca != cb {
                return false;
            }
            if ca == 0 {
                return true;
            }
            i += 1;
        }
        true
    }
}

/// Resolves `name` against the long option table.
///
/// # Safety
///
/// `name` must point to a valid long option name and `longopts` must be either null or a valid
/// zero-terminated long option table.
unsafe fn resolve_long_option(
    name: *const c_char,
    namelen: usize,
    longopts: *const LongOption,
) -> LongOptionMatch {
    unsafe {
        if namelen == 0 {
            return LongOptionMatch::NoMatch;
        }

        let mut matched: Option<usize> = None;
        let mut nmatch: u32 = 0;
        if !longopts.is_null() {
            let mut i: usize = 0;
            loop {
                let opt_name: *const c_char = (*longopts.add(i)).name;
                if opt_name.is_null() {
                    break;
                }
                if prefix_equals(name, opt_name, namelen) {
                    if c_strlen(opt_name) == namelen {
                        return LongOptionMatch::Matched(i);
                    }
                    matched = Some(i);
                    nmatch += 1;
                }
                i += 1;
            }
        }

        match matched {
            Some(m) if nmatch == 1 => LongOptionMatch::Matched(m),
            Some(_) => LongOptionMatch::Ambiguous,
            None => LongOptionMatch::NoMatch,
        }
    }
}

/// Consumes a resolved long option and updates global parser state.
///
/// # Safety
///
/// The pointers in `context` and `option` must satisfy [`getopt_long`]'s safety contract, and
/// `option.matched` must be a valid index into `context.longopts`.
unsafe fn consume_long_option(context: &LongOptionContext, option: ResolvedLongOption) -> c_int {
    unsafe {
        let o: *const LongOption = context.longopts.add(option.matched);
        optind += 1;
        if !context.longindex.is_null() {
            *context.longindex = c_int::try_from(option.matched).unwrap_or(0);
        }

        let has_arg: c_int = (*o).has_arg;
        if has_arg == REQUIRED_ARGUMENT {
            if let Some(pos) = option.eq {
                optarg = option.name.add(pos + 1);
            } else if optind < context.argc {
                optarg = *context.argv.add(optind as usize);
                optind += 1;
            } else {
                optopt = (*o).val;
                return if *context.optstring == COLON {
                    b':' as c_int
                } else {
                    QUESTION
                };
            }
        } else if has_arg == OPTIONAL_ARGUMENT {
            optarg = match option.eq {
                Some(pos) => option.name.add(pos + 1),
                None => ::core::ptr::null_mut(),
            };
        } else {
            // NO_ARGUMENT: reject an inline "=value".
            optarg = ::core::ptr::null_mut();
            if option.eq.is_some() {
                optopt = (*o).val;
                return QUESTION;
            }
        }

        if !(*o).flag.is_null() {
            *(*o).flag = (*o).val;
            return 0;
        }
        (*o).val
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Parses command-line options from `argv`, accepting GNU-style long options (`"--name"`,
/// `"--name=value"`, and `"--name value"`) in addition to the short options understood by
/// [`getopt`]. Long option names may be abbreviated to any unambiguous prefix. Anything that is not
/// a `"--"`-prefixed long option is delegated to [`getopt`]. The global `optarg`, `optind`, and
/// `optopt` variables are updated exactly as for [`getopt`].
///
/// # Parameters
///
/// - `argc`: Argument count.
/// - `argv`: Argument vector.
/// - `optstring`: Recognized short-option characters (see [`getopt`]).
/// - `longopts`: Array of [`LongOption`] entries terminated by an all-zero entry, or null.
/// - `longindex`: If non-null, receives the index of the matched long option in `longopts`.
///
/// # Returns
///
/// The matched option's `val` (or `0` when it stores through `flag`), the short-option character,
/// `'?'` for an unknown or ambiguous option, `':'` for a missing argument when `optstring` begins
/// with `':'`, or `-1` when option parsing is complete.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and modifies global state. `argv`
/// must contain `argc` valid, null-terminated strings, `optstring` must be a valid null-terminated
/// string, and `longopts` (when non-null) must be a valid array terminated by a zero `name` entry.
/// The global `getopt` state must not be accessed concurrently from another thread.
///
/// # References
///
/// - <https://www.gnu.org/software/libc/manual/html_node/Getopt-Long-Options.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getopt_long(
    argc: c_int,
    argv: *const *mut c_char,
    optstring: *const c_char,
    longopts: *const LongOption,
    longindex: *mut c_int,
) -> c_int {
    unsafe {
        let context: LongOptionContext = LongOptionContext {
            argc,
            argv,
            optstring,
            longopts,
            longindex,
        };

        // GNU/glibc reset convention (mirrors getopt()): a caller restarts option processing by
        // setting `optind` to 0. Rewind to the first argument and clear the short-option cursor so
        // a subsequent delegation to getopt() does not resume a stale clustered element.
        if optind == 0 {
            optind = 1;
            reset_short_option_cursor();
        }

        let idx: c_int = optind;
        if idx < 0 || idx >= argc {
            return -1;
        }

        let arg: *mut c_char = *argv.add(idx as usize);

        // Not a "--name" long option (short option, "-", or "--"): delegate to getopt().
        if *arg != DASH || *arg.add(1) != DASH || *arg.add(2) == 0 {
            return getopt(argc, argv, optstring);
        }

        let name: *mut c_char = arg.add(2);
        let eq: Option<usize> = find_equals(name);
        let namelen: usize = match eq {
            Some(pos) => pos,
            None => c_strlen(name),
        };

        let matched: usize = match resolve_long_option(name, namelen, longopts) {
            LongOptionMatch::Matched(m) => m,
            LongOptionMatch::Ambiguous | LongOptionMatch::NoMatch => {
                optind += 1;
                optopt = 0;
                return QUESTION;
            },
        };

        consume_long_option(&context, ResolvedLongOption { name, eq, matched })
    }
}

///
/// # Description
///
/// Parses command-line options like [`getopt_long`]. The GNU `getopt_long_only` variant also
/// accepts long options introduced by a single `'-'`. A single-dash option that does not match any
/// long option is delegated to [`getopt`] and parsed as a short option.
///
/// # Parameters
///
/// See [`getopt_long`].
///
/// # Returns
///
/// See [`getopt_long`].
///
/// # Safety
///
/// See [`getopt_long`].
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getopt_long_only(
    argc: c_int,
    argv: *const *mut c_char,
    optstring: *const c_char,
    longopts: *const LongOption,
    longindex: *mut c_int,
) -> c_int {
    unsafe {
        let context: LongOptionContext = LongOptionContext {
            argc,
            argv,
            optstring,
            longopts,
            longindex,
        };

        if optind == 0 {
            optind = 1;
            reset_short_option_cursor();
        }

        let idx: c_int = optind;
        if idx < 0 || idx >= argc {
            return -1;
        }

        let arg: *mut c_char = *argv.add(idx as usize);
        if *arg != DASH || *arg.add(1) == 0 {
            return getopt(argc, argv, optstring);
        }

        let double_dash: bool = *arg.add(1) == DASH;
        if double_dash && *arg.add(2) == 0 {
            return getopt(argc, argv, optstring);
        }

        let name: *mut c_char = if double_dash { arg.add(2) } else { arg.add(1) };
        let eq: Option<usize> = find_equals(name);
        let namelen: usize = match eq {
            Some(pos) => pos,
            None => c_strlen(name),
        };

        let matched: usize = match resolve_long_option(name, namelen, longopts) {
            LongOptionMatch::Matched(m) => m,
            LongOptionMatch::Ambiguous => {
                optind += 1;
                optopt = 0;
                return QUESTION;
            },
            LongOptionMatch::NoMatch if !double_dash => return getopt(argc, argv, optstring),
            LongOptionMatch::NoMatch => {
                optind += 1;
                optopt = 0;
                return QUESTION;
            },
        };

        consume_long_option(&context, ResolvedLongOption { name, eq, matched })
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{
        getopt_long,
        getopt_long_only,
        LongOption,
        NO_ARGUMENT,
        OPTIONAL_ARGUMENT,
        REQUIRED_ARGUMENT,
    };
    use crate::unistd::bindings::getopt::{
        optarg,
        optind,
    };
    use ::std::{
        ffi::{
            CStr,
            CString,
        },
        ptr,
        sync::Mutex,
        vec::Vec,
    };
    use ::sysapi::ffi::{
        c_char,
        c_int,
    };

    /// Serializes access to the process-wide `getopt` globals across parallel tests.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Owns the C strings backing an argument vector for the duration of a test.
    struct Argv {
        _storage: Vec<CString>,
        ptrs: Vec<*mut c_char>,
    }

    impl Argv {
        fn new(args: &[&str]) -> Self {
            let storage: Vec<CString> = args
                .iter()
                .map(|a| CString::new(*a).expect("no interior nul"))
                .collect();
            let ptrs: Vec<*mut c_char> = storage.iter().map(|s| s.as_ptr().cast_mut()).collect();
            Self {
                _storage: storage,
                ptrs,
            }
        }

        fn argc(&self) -> c_int {
            c_int::try_from(self.ptrs.len()).expect("fits")
        }

        fn argv(&self) -> *const *mut c_char {
            self.ptrs.as_ptr()
        }
    }

    /// Builds a long-option table terminated by a zero entry.
    fn longopts(entries: &[(&CStr, c_int, c_int)]) -> Vec<LongOption> {
        let mut v: Vec<LongOption> = entries
            .iter()
            .map(|(name, has_arg, val)| LongOption {
                name: name.as_ptr(),
                has_arg: *has_arg,
                flag: ptr::null_mut(),
                val: *val,
            })
            .collect();
        v.push(LongOption {
            name: ptr::null(),
            has_arg: 0,
            flag: ptr::null_mut(),
            val: 0,
        });
        v
    }

    /// Reads back `optarg` as an owned string, or `None` when it is null.
    fn optarg_str() -> Option<::std::string::String> {
        let p: *mut c_char = unsafe { optarg };
        if p.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
        }
    }

    #[test]
    fn matches_long_option_with_separate_argument() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--output", "file.txt", "rest"]);
        let name: CString = CString::new("output").expect("no nul");
        let opts: Vec<LongOption> =
            longopts(&[(name.as_c_str(), REQUIRED_ARGUMENT, b'o' as c_int)]);
        let optstring: CString = CString::new("o:").expect("no nul");
        let mut longindex: c_int = -1;

        let rc: c_int = unsafe {
            getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), &mut longindex)
        };

        assert_eq!(rc, b'o' as c_int);
        assert_eq!(optarg_str().as_deref(), Some("file.txt"));
        assert_eq!(longindex, 0);
        assert_eq!(unsafe { optind }, 3);
    }

    #[test]
    fn optind_zero_restarts_long_option_processing() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--verbose", "rest"]);
        let name: CString = CString::new("verbose").expect("no nul");
        let opts: Vec<LongOption> = longopts(&[(name.as_c_str(), NO_ARGUMENT, b'v' as c_int)]);
        let optstring: CString = CString::new("v").expect("no nul");

        // First pass matches the long option and then stops at the operand.
        assert_eq!(
            unsafe {
                getopt_long(
                    argv.argc(),
                    argv.argv(),
                    optstring.as_ptr(),
                    opts.as_ptr(),
                    ptr::null_mut(),
                )
            },
            b'v' as c_int
        );
        assert_eq!(
            unsafe {
                getopt_long(
                    argv.argc(),
                    argv.argv(),
                    optstring.as_ptr(),
                    opts.as_ptr(),
                    ptr::null_mut(),
                )
            },
            -1
        );
        assert_eq!(unsafe { optind }, 2);

        // Setting optind to 0 (GNU reset convention) restarts parsing from the first argument.
        unsafe {
            optind = 0;
        }
        assert_eq!(
            unsafe {
                getopt_long(
                    argv.argc(),
                    argv.argv(),
                    optstring.as_ptr(),
                    opts.as_ptr(),
                    ptr::null_mut(),
                )
            },
            b'v' as c_int
        );
        assert_eq!(unsafe { optind }, 2);
    }

    #[test]
    fn matches_long_option_with_inline_argument() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--output=file.txt"]);
        let name: CString = CString::new("output").expect("no nul");
        let opts: Vec<LongOption> =
            longopts(&[(name.as_c_str(), REQUIRED_ARGUMENT, b'o' as c_int)]);
        let optstring: CString = CString::new("o:").expect("no nul");

        let rc: c_int = unsafe {
            getopt_long(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };

        assert_eq!(rc, b'o' as c_int);
        assert_eq!(optarg_str().as_deref(), Some("file.txt"));
    }

    #[test]
    fn accepts_unambiguous_prefix() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--verb"]);
        let verbose: CString = CString::new("verbose").expect("no nul");
        let help: CString = CString::new("help").expect("no nul");
        let opts: Vec<LongOption> = longopts(&[
            (verbose.as_c_str(), NO_ARGUMENT, b'v' as c_int),
            (help.as_c_str(), NO_ARGUMENT, b'h' as c_int),
        ]);
        let optstring: CString = CString::new("vh").expect("no nul");

        let rc: c_int = unsafe {
            getopt_long(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };

        assert_eq!(rc, b'v' as c_int);
    }

    #[test]
    fn rejects_ambiguous_prefix() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--ver"]);
        let verbose: CString = CString::new("verbose").expect("no nul");
        let version: CString = CString::new("version").expect("no nul");
        let opts: Vec<LongOption> = longopts(&[
            (verbose.as_c_str(), NO_ARGUMENT, b'v' as c_int),
            (version.as_c_str(), NO_ARGUMENT, b'V' as c_int),
        ]);
        let optstring: CString = CString::new("vV").expect("no nul");

        let rc: c_int = unsafe {
            getopt_long(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };

        assert_eq!(rc, b'?' as c_int);
    }

    #[test]
    fn rejects_empty_long_option_name() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--=value"]);
        let output: CString = CString::new("output").expect("no nul");
        let opts: Vec<LongOption> = longopts(&[(output.as_c_str(), REQUIRED_ARGUMENT, 1000)]);
        let optstring: CString = CString::new("").expect("no nul");

        let rc: c_int = unsafe {
            getopt_long(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };

        assert_eq!(rc, b'?' as c_int);
        assert_eq!(unsafe { optind }, 2);
    }

    #[test]
    fn optional_argument_present_and_absent() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let color: CString = CString::new("color").expect("no nul");
        let optstring: CString = CString::new("").expect("no nul");

        // Present (inline).
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--color=always"]);
        let opts: Vec<LongOption> =
            longopts(&[(color.as_c_str(), OPTIONAL_ARGUMENT, b'c' as c_int)]);
        let rc: c_int = unsafe {
            getopt_long(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };
        assert_eq!(rc, b'c' as c_int);
        assert_eq!(optarg_str().as_deref(), Some("always"));

        // Absent.
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "--color"]);
        let rc: c_int = unsafe {
            getopt_long(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };
        assert_eq!(rc, b'c' as c_int);
        assert_eq!(optarg_str(), None);
    }

    #[test]
    fn delegates_short_options_to_getopt() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "-v"]);
        let verbose: CString = CString::new("verbose").expect("no nul");
        let opts: Vec<LongOption> = longopts(&[(verbose.as_c_str(), NO_ARGUMENT, b'v' as c_int)]);
        let optstring: CString = CString::new("v").expect("no nul");

        let rc: c_int = unsafe {
            getopt_long(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };

        assert_eq!(rc, b'v' as c_int);
    }

    #[test]
    fn long_only_accepts_single_dash_long_option() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "-verbose"]);
        let verbose: CString = CString::new("verbose").expect("no nul");
        let opts: Vec<LongOption> = longopts(&[(verbose.as_c_str(), NO_ARGUMENT, 1000)]);
        let optstring: CString = CString::new("v").expect("no nul");

        let rc: c_int = unsafe {
            getopt_long_only(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };

        assert_eq!(rc, 1000);
        assert_eq!(unsafe { optind }, 2);
    }

    #[test]
    fn long_only_delegates_unmatched_single_dash_option_to_getopt() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            optind = 1;
        }
        let argv: Argv = Argv::new(&["prog", "-v"]);
        let output: CString = CString::new("output").expect("no nul");
        let opts: Vec<LongOption> = longopts(&[(output.as_c_str(), NO_ARGUMENT, b'o' as c_int)]);
        let optstring: CString = CString::new("v").expect("no nul");

        let rc: c_int = unsafe {
            getopt_long_only(
                argv.argc(),
                argv.argv(),
                optstring.as_ptr(),
                opts.as_ptr(),
                ptr::null_mut(),
            )
        };

        assert_eq!(rc, b'v' as c_int);
        assert_eq!(unsafe { optind }, 2);
    }
}
