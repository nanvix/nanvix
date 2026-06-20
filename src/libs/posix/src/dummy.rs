// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_long,
    c_void,
};
use ::syslog::trace_libcall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Opens a process by creating a pipe, forking, and invoking the shell.
///
/// # Parameters
///
/// - `command`: Null-terminated string containing the command to be executed.
/// - `mode`: Null-terminated string that specifies the mode for the pipe (e.g., "r" or "w").
///
/// # Returns
///
/// On success, returns a non-null pointer to an opaque stream object. On failure, returns a null
/// pointer and sets `errno` to indicate the error.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// A future implementation should create the appropriate pipe, fork a child process, and execute
/// the requested command in a POSIX-compatible shell environment.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers.
/// It is safe to call this function if `command` and `mode` (when non-null) point to valid
/// null-terminated C strings.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn popen(command: *const c_char, mode: *const c_char) -> *mut c_void {
    ::syslog::debug!("popen(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    core::ptr::null_mut()
}

///
/// # Description
///
/// Closes a stream opened by [`popen()`] and waits for the associated process to terminate.
///
/// # Parameters
///
/// - `stream`: Pointer previously returned by [`popen()`].
///
/// # Returns
///
/// On success, returns the termination status of the command. On failure, returns `-1` and sets
/// `errno` to indicate the error.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// A future implementation should close any pipe file descriptors, reap the child process, and
/// return its status code in a POSIX-compatible manner.
///
/// # Safety
///
/// This function is unsafe because it operates on an opaque raw pointer supplied by foreign
/// callers. It is safe to call this function if `stream` is either null or a value previously
/// returned by [`popen()`] in a future, fully implemented version.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pclose(stream: *mut c_void) -> c_int {
    ::syslog::debug!("pclose(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Retrieves the parameters associated with the terminal referred to by `fd` and stores them in
/// the structure pointed to by `termios_p`.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to a terminal device.
/// - `termios_p`: Pointer to a buffer where the terminal attributes are stored on success.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error.
///
/// # Notes
///
/// In standalone mode this is `ioctl(fd, TCGETS, termios_p)`, backed by the vfsd console terminal.
/// Hosted deployments have no guest terminal device, so the call fails with `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers. It is
/// safe to call this function if `termios_p` points to a valid, writable `struct termios`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn tcgetattr(fd: c_int, termios_p: *mut c_void) -> c_int {
    #[cfg(feature = "standalone")]
    {
        match unsafe { ::syscall::sys::ioctl::ioctl(fd, ::sysapi::sys_ioctl::TCGETS, termios_p) } {
            Ok(_) => 0,
            Err(error) => {
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
    #[cfg(not(feature = "standalone"))]
    {
        let _ = (fd, termios_p);
        ::syslog::debug!("tcgetattr(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}

///
/// # Description
///
/// Sets the parameters associated with the terminal referred to by `fd` according to the values
/// in the structure pointed to by `termios_p`.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to a terminal device.
/// - `optional_actions`: How the change is applied (e.g., `TCSANOW`, `TCSADRAIN`, `TCSAFLUSH`).
/// - `termios_p`: Pointer to a buffer containing the desired terminal attributes.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error.
///
/// # Notes
///
/// In standalone mode this is `ioctl(fd, TCSETS, termios_p)`, backed by the vfsd console terminal.
/// The console has no output queue to drain or input queue to flush, so `optional_actions` modes
/// (`TCSANOW`/`TCSADRAIN`/`TCSAFLUSH`) are equivalent and the change is applied immediately. Hosted
/// deployments have no guest terminal device, so the call fails with `ENOSYS`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers. It is
/// safe to call this function if `termios_p` points to a valid, readable `struct termios`.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn tcsetattr(
    fd: c_int,
    optional_actions: c_int,
    termios_p: *const c_void,
) -> c_int {
    #[cfg(feature = "standalone")]
    {
        match optional_actions {
            ::sysapi::termios::TCSANOW
            | ::sysapi::termios::TCSADRAIN
            | ::sysapi::termios::TCSAFLUSH => {},
            _ => {
                unsafe {
                    *__errno_location() = ErrorCode::InvalidArgument.get();
                }
                return -1;
            },
        }
        match unsafe {
            ::syscall::sys::ioctl::ioctl(fd, ::sysapi::sys_ioctl::TCSETS, termios_p as *mut c_void)
        } {
            Ok(_) => 0,
            Err(error) => {
                unsafe {
                    *__errno_location() = error.code.get();
                }
                -1
            },
        }
    }
    #[cfg(not(feature = "standalone"))]
    {
        let _ = (fd, optional_actions, termios_p);
        ::syslog::debug!("tcsetattr(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}

///
/// # Description
///
/// Resolves a pathname to an absolute, canonical form, eliminating symbolic links, `.` and `..`
/// components.
///
/// # Parameters
///
/// - `path`: Null-terminated input pathname to resolve.
/// - `resolved_path`: Optional caller-provided buffer where the resolved path would be stored. If
///   null, a future implementation would allocate a new buffer via the POSIX-specified allocator.
///
/// # Returns
///
/// On success, returns a pointer to the resolved path (either `resolved_path` or an allocated
/// buffer). On failure, returns null and sets `errno` to indicate the error.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// A future implementation should perform path normalization, handle symbolic links (with an
/// upper bound on link depth to avoid cycles), and ensure the result does not exceed `PATH_MAX`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers. It is
/// safe to call this function if `path` points to a valid, null-terminated C string and
/// `resolved_path` is either null or points to a writable buffer large enough to hold the
/// canonical path in a future, fully implemented version.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn realpath(path: *const c_char, resolved_path: *mut c_char) -> *mut c_char {
    ::syslog::debug!("realpath(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    core::ptr::null_mut()
}

///
/// # Description
///
/// Retrieves the value of a configurable system limit or option associated with the
/// pathname `path`, as identified by `name` (one of the `_PC_*` selectors defined in
/// `<unistd.h>`).
///
/// # Parameters
///
/// - `path`: Null-terminated pathname of the file or directory being queried.
/// - `name`: A `_PC_*` selector specifying which configurable value to retrieve.
///
/// # Returns
///
/// On success a non-negative limit value, or `-1` (with `errno` unchanged) when the
/// queried option has no determinate limit. On failure returns `-1` and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always returns `-1` with `errno = ENOSYS`,
/// matching the convention used by the other "not implemented" stubs in this module.
/// Callers (notably libstdc++'s `std::filesystem`) treat `-1` as "no limit known" and
/// fall back to compile-time defaults such as `PATH_MAX`, so this stub is sufficient
/// to satisfy the libstdc++ link without changing behaviour. A future implementation
/// should return real limits for the selectors it knows about (e.g. `_PC_PATH_MAX`,
/// `_PC_NAME_MAX`, `_PC_LINK_MAX`), and only set `errno = EINVAL` for genuinely
/// unrecognised selectors per the POSIX contract.
///
/// # Safety
///
/// This function is unsafe because it accepts a raw pointer supplied by foreign callers.
/// It is safe to call this function if `path` (when non-null) points to a valid
/// null-terminated C string.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn pathconf(_path: *const c_char, _name: c_int) -> c_long {
    ::syslog::debug!("pathconf(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Retrieves the value of a configurable system limit or option associated with the
/// open file descriptor `fd`, as identified by `name` (one of the `_PC_*` selectors
/// defined in `<unistd.h>`).
///
/// # Parameters
///
/// - `fd`: An open file descriptor to query.
/// - `name`: A `_PC_*` selector specifying which configurable value to retrieve.
///
/// # Returns
///
/// On success a non-negative limit value, or `-1` (with `errno` unchanged) when the
/// queried option has no determinate limit. On failure returns `-1` and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always returns `-1` with `errno = ENOSYS`.
/// See `pathconf()` for the rationale on why this stub is acceptable for the current
/// libstdc++ link requirements.
///
/// # Safety
///
/// This function is safe to call with any integer; passing a descriptor that is not
/// currently open does not change the (stub) behaviour.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn fpathconf(_fd: c_int, _name: c_int) -> c_long {
    ::syslog::debug!("fpathconf(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Performs a file tree walk starting at `dirpath` and calls the user-supplied callback
/// function `fn_cb` for each encountered file or directory. The walk is performed in
/// unspecified order and is limited by `nopenfd`, which specifies the maximum number of
/// file descriptors that may be used simultaneously during the traversal.
///
/// # Parameters
///
/// - `dirpath`: Null-terminated path to the starting directory.
/// - `fn_cb`: Callback invoked for each entry. Receives: the entry path, a pointer to a
///   `stat`-like structure (platform specific) and a type flag describing the entry kind.
/// - `nopenfd`: Maximum number of file descriptors to keep open while traversing.
///
/// # Returns
///
/// Returns `0` on success. On failure it returns `-1` and sets `errno` to indicate the
/// error. If the callback returns a non-zero value, a future compliant implementation
/// would stop the walk and propagate that value as the return code of `ftw()`.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not
/// implemented). A future implementation should:
/// - Perform a depth-first (or specified order) traversal of the directory tree.
/// - Invoke the callback for each file, directory, symbolic link, etc.
/// - Enforce `nopenfd` by closing directories when descending beyond the limit.
/// - Populate and pass a proper `stat` structure to the callback.
/// - Map filesystem errors to appropriate `errno` values and continue or abort
///   traversal according to specification and callback return values.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign
/// callers and invokes a function pointer. It is safe to call this function if `dirpath`
/// is a valid, null-terminated string and `fn_cb` (when non-null) points to a callable
/// function with the expected signature in a future, fully implemented version.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn ftw(
    dirpath: *const c_char,
    fn_cb: Option<unsafe extern "C" fn(*const c_char, *const c_void, c_int) -> c_int>,
    nopenfd: c_int,
) -> c_int {
    ::syslog::debug!("ftw(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Retrieves file-system statistics for the file system that contains the file named by
/// `path` and stores them in the `statvfs` structure pointed to by `buf`.
///
/// # Parameters
///
/// - `path`: Null-terminated pathname of any file within the queried file system.
/// - `buf`: Pointer to a `struct statvfs` to be filled in on success.
///
/// # Returns
///
/// On success returns `0` and populates `*buf`. On failure returns `-1` and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// It exists so that consumers which only reference the symbol (notably libstdc++'s
/// `std::filesystem::space()`) link successfully; such callers treat the `-1`/`errno`
/// failure as "information unavailable". A future implementation should query the backing
/// file-system daemon and populate the block / inode counts and the mount flags.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers supplied by foreign callers.
/// It is safe to call this function if `path` points to a valid, null-terminated C string
/// and `buf` (when non-null) points to writable storage large enough for a `struct statvfs`
/// in a future, fully implemented version.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn statvfs(_path: *const c_char, _buf: *mut c_void) -> c_int {
    ::syslog::debug!("statvfs(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

///
/// # Description
///
/// Opens a directory stream positioned at the first entry, for the directory associated with
/// the already-open file descriptor `fd`.
///
/// # Parameters
///
/// - `fd`: An open file descriptor referring to a directory.
///
/// # Returns
///
/// On success returns a pointer to an opaque `DIR` stream object. On failure returns a null
/// pointer and sets `errno`.
///
/// # Notes
///
/// This is a dummy implementation that always fails with `ENOSYS` (function not implemented).
/// It exists so that consumers which only reference the symbol (notably libstdc++'s
/// `std::filesystem` directory iterators) link successfully; such callers treat the null
/// return as "directory could not be opened". A future implementation should take ownership of
/// `fd` and return a `DIR` stream backed by the directory it refers to.
///
/// # Safety
///
/// This function is safe to call with any integer; passing a descriptor that is not currently
/// open does not change the (stub) behaviour.
///
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn fdopendir(_fd: c_int) -> *mut c_void {
    ::syslog::debug!("fdopendir(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    core::ptr::null_mut()
}
