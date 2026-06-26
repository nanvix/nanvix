// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::syscall::{
    pw_dir_copy_len,
    resolve_gid,
    PW_DIR_CAP,
};
use ::core::{
    ffi::CStr,
    ptr,
};
use ::sysapi::{
    ffi::c_char,
    pwd::passwd,
    sys_types::{
        gid_t,
        uid_t,
    },
};
use ::syslog::trace_libcall;

//==================================================================================================
// Static Storage
//==================================================================================================

/// Username reported for the single-user Nanvix system.
static PW_NAME: [u8; 5] = *b"root\0";

/// Placeholder password field (POSIX convention is a single `x`).
static PW_PASSWD: [u8; 2] = *b"x\0";

/// GECOS (real-name) field. Empty on Nanvix.
static PW_GECOS: [u8; 1] = *b"\0";

/// Login shell.
static PW_SHELL: [u8; 8] = *b"/bin/sh\0";

/// Name of the environment variable that holds the home directory.
static HOME_KEY: [u8; 5] = *b"HOME\0";

/// Backing storage for the home directory returned in `pw_dir`.
static mut PW_DIR: [u8; PW_DIR_CAP] = [0u8; PW_DIR_CAP];

/// Static `passwd` entry returned by [`getpwuid`]. POSIX allows the result to
/// point at static storage that is overwritten by subsequent calls.
static mut PW_ENTRY: passwd = passwd {
    pw_name: ptr::null(),
    pw_passwd: ptr::null(),
    pw_uid: 0,
    pw_gid: 0,
    pw_gecos: ptr::null(),
    pw_dir: ptr::null(),
    pw_shell: ptr::null(),
};

//==================================================================================================
// External Functions
//==================================================================================================

unsafe extern "C" {
    /// Reads a value from the process environment table populated at start-of-day.
    /// Provided by the stdlib bindings; declared here so this module does not need a
    /// direct dependency on the defining crate.
    fn getenv(name: *const c_char) -> *mut c_char;
}

//==================================================================================================
// getpwuid()
//==================================================================================================

///
/// # Description
///
/// Returns the password database entry for the user identified by `uid`. Nanvix is
/// effectively a single-user system, so rather than consulting an `/etc/passwd`
/// database this function returns a statically-allocated [`passwd`] entry describing
/// the `root` user. The home directory (`pw_dir`) is taken from the `HOME` environment
/// variable when set, and falls back to `"/"` otherwise. The supplied `uid` is reported
/// verbatim in `pw_uid`, and `pw_gid` is populated from the real group ID returned by
/// `getgid()`, falling back to `0` (root) when that lookup fails.
///
/// # Parameters
///
/// - `uid`: The user ID to report in the returned entry.
///
/// # Returns
///
/// A pointer to a statically-allocated [`passwd`] entry. Per POSIX, the storage may be
/// overwritten by subsequent calls.
///
/// # Safety
///
/// This function returns a pointer to static storage that must not be freed by the
/// caller and may be overwritten by subsequent calls to this function. It is not
/// thread-safe: callers must ensure no other thread calls `getpwuid()` while the
/// returned pointer is still in use.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getpwuid(uid: uid_t) -> *mut passwd {
    // Resolve the home directory from the environment (Layer 2), falling back to "/"
    // when HOME is unset or holds an invalid (e.g. empty) value.
    let home_ptr: *mut c_char = unsafe { getenv(HOME_KEY.as_ptr() as *const c_char) };
    let home_bytes: &[u8] = if home_ptr.is_null() {
        b"/"
    } else {
        // SAFETY: `getenv()` returns either null or a pointer to a NUL-terminated string
        // that lives in the process environment table.
        let bytes: &[u8] = unsafe { CStr::from_ptr(home_ptr) }.to_bytes();
        if bytes.is_empty() {
            b"/"
        } else {
            bytes
        }
    };

    // Resolve the real group ID, falling back to 0 (root) on error.
    let gid: gid_t = resolve_gid();

    // Copy the home directory into the static buffer, truncating to fit and always
    // NUL-terminating. Raw-pointer writes are used to avoid taking references to the
    // mutable static (see `static_mut_refs`).
    let n: usize = pw_dir_copy_len(home_bytes);
    let dir: *mut u8 = ptr::addr_of_mut!(PW_DIR) as *mut u8;
    // SAFETY: `n <= PW_DIR_CAP - 1`, so the copy and the terminating NUL stay within the
    // bounds of `PW_DIR`. The source and destination do not overlap.
    unsafe {
        ptr::copy_nonoverlapping(home_bytes.as_ptr(), dir, n);
        dir.add(n).write(0);
    }

    // Populate the static entry through a raw pointer. `addr_of_mut!` avoids constructing a
    // reference to the mutable static (see `static_mut_refs`). `passwd` is `#[repr(C)]`, so the
    // static is naturally aligned and plain field stores produce a correctly-aligned
    // `*mut passwd` for C callers.
    let entry: *mut passwd = ptr::addr_of_mut!(PW_ENTRY);
    // SAFETY: `entry` points to the live, aligned `PW_ENTRY` static and the string pointers refer
    // to NUL-terminated static buffers that outlive any use of the returned entry.
    unsafe {
        (*entry).pw_name = PW_NAME.as_ptr() as *const c_char;
        (*entry).pw_passwd = PW_PASSWD.as_ptr() as *const c_char;
        (*entry).pw_uid = uid;
        (*entry).pw_gid = gid;
        (*entry).pw_gecos = PW_GECOS.as_ptr() as *const c_char;
        (*entry).pw_dir = ptr::addr_of!(PW_DIR) as *const c_char;
        (*entry).pw_shell = PW_SHELL.as_ptr() as *const c_char;
    }

    entry
}

//==================================================================================================
// getpwnam()
//==================================================================================================

///
/// # Description
///
/// Returns the password database entry for the user with the login name `name`. Nanvix is
/// effectively a single-user system whose only account is `root`, so the lookup ignores `name` and
/// returns the same statically-allocated [`passwd`] entry produced by [`getpwuid`] for the root
/// user (UID `0`).
///
/// # Parameters
///
/// - `name`: The login name to look up. Ignored on Nanvix.
///
/// # Returns
///
/// A pointer to a statically-allocated [`passwd`] entry describing the `root` user. Per POSIX, the
/// storage may be overwritten by subsequent calls.
///
/// # Safety
///
/// This function returns a pointer to static storage that must not be freed by the caller and may
/// be overwritten by subsequent calls. It is not thread-safe: callers must ensure no other thread
/// calls `getpwnam()` or `getpwuid()` while the returned pointer is still in use.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getpwnam(name: *const c_char) -> *mut passwd {
    // Nanvix has a single account (`root`, UID 0); the looked-up name is ignored.
    let _ = name;
    unsafe { getpwuid(0) }
}
