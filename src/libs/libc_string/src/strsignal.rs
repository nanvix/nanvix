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
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns a pointer to a string describing the signal number `sig`. The returned string does not
/// include a trailing newline. Signal numbers follow the standard Nanvix assignment declared in
/// `<signal.h>`. Unknown signal numbers map to the generic description `"Unknown signal"`.
///
/// # Parameters
///
/// - `sig`: The signal number to describe.
///
/// # Return Value
///
/// A pointer to a statically-allocated, NUL-terminated description string. The caller must not
/// modify or free the returned string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn strsignal(sig: c_int) -> *mut c_char {
    let message: &[u8] = match sig {
        1 => b"Hangup\0",                    // SIGHUP
        2 => b"Interrupt\0",                 // SIGINT
        3 => b"Quit\0",                      // SIGQUIT
        4 => b"Illegal instruction\0",       // SIGILL
        5 => b"Trace/breakpoint trap\0",     // SIGTRAP
        6 => b"Aborted\0",                   // SIGABRT
        7 => b"Bus error\0",                 // SIGBUS
        8 => b"Floating point exception\0",  // SIGFPE
        9 => b"Killed\0",                    // SIGKILL
        10 => b"User defined signal 1\0",    // SIGUSR1
        11 => b"Segmentation fault\0",       // SIGSEGV
        12 => b"User defined signal 2\0",    // SIGUSR2
        13 => b"Broken pipe\0",              // SIGPIPE
        14 => b"Alarm clock\0",              // SIGALRM
        15 => b"Terminated\0",               // SIGTERM
        17 => b"Child exited\0",             // SIGCHLD
        18 => b"Continued\0",                // SIGCONT
        19 => b"Stopped (signal)\0",         // SIGSTOP
        20 => b"Stopped\0",                  // SIGTSTP
        21 => b"Stopped (tty input)\0",      // SIGTTIN
        22 => b"Stopped (tty output)\0",     // SIGTTOU
        23 => b"Urgent I/O condition\0",     // SIGURG
        24 => b"CPU time limit exceeded\0",  // SIGXCPU
        25 => b"File size limit exceeded\0", // SIGXFSZ
        26 => b"Virtual timer expired\0",    // SIGVTALRM
        27 => b"Profiling timer expired\0",  // SIGPROF
        28 => b"Window changed\0",           // SIGWINCH
        29 => b"I/O possible\0",             // SIGIO
        31 => b"Bad system call\0",          // SIGSYS
        _ => b"Unknown signal\0",
    };

    message.as_ptr().cast::<c_char>().cast_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strsignal;
    use ::core::ffi::CStr;
    use ::sysapi::ffi::c_char;

    fn describe(sig: i32) -> ::std::string::String {
        let ptr: *mut c_char = strsignal(sig);
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn test_known_signal() {
        assert_eq!(describe(9), "Killed");
        assert_eq!(describe(15), "Terminated");
    }

    #[test]
    fn test_unknown_signal() {
        assert_eq!(describe(12345), "Unknown signal");
    }
}
