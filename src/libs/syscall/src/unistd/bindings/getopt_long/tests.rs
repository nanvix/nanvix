// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    let opts: Vec<LongOption> = longopts(&[(name.as_c_str(), REQUIRED_ARGUMENT, b'o' as c_int)]);
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
    let opts: Vec<LongOption> = longopts(&[(name.as_c_str(), REQUIRED_ARGUMENT, b'o' as c_int)]);
    let optstring: CString = CString::new("o:").expect("no nul");

    let rc: c_int = unsafe {
        getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), ptr::null_mut())
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
        getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), ptr::null_mut())
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
        getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), ptr::null_mut())
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
        getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), ptr::null_mut())
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
    let opts: Vec<LongOption> = longopts(&[(color.as_c_str(), OPTIONAL_ARGUMENT, b'c' as c_int)]);
    let rc: c_int = unsafe {
        getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), ptr::null_mut())
    };
    assert_eq!(rc, b'c' as c_int);
    assert_eq!(optarg_str().as_deref(), Some("always"));

    // Absent.
    unsafe {
        optind = 1;
    }
    let argv: Argv = Argv::new(&["prog", "--color"]);
    let rc: c_int = unsafe {
        getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), ptr::null_mut())
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
        getopt_long(argv.argc(), argv.argv(), optstring.as_ptr(), opts.as_ptr(), ptr::null_mut())
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
