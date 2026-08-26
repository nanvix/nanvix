// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::syscall::{
    pw_dir_copy_len,
    PW_DIR_CAP,
};
use ::core::{
    cell::UnsafeCell,
    ffi::CStr,
    ptr,
};
use ::sys::pm::{
    GroupIdentifier,
    UserIdentifier,
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

/// Name of the synthetic root user.
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
static mut PW_DIR: [u8; PW_DIR_CAP] = {
    let mut dir: [u8; PW_DIR_CAP] = [0u8; PW_DIR_CAP];
    dir[0] = b'/';
    dir
};

/// Shared storage for the synthetic root entry.
struct PasswdEntry(UnsafeCell<passwd>);

// SAFETY: callers must serialize access to the POSIX static entry.
unsafe impl Sync for PasswdEntry {}

/// Static root entry returned by [`getpwuid`] and [`getpwnam`].
static PW_ENTRY: PasswdEntry = PasswdEntry(UnsafeCell::new(passwd {
    pw_name: PW_NAME.as_ptr() as *const c_char,
    pw_passwd: PW_PASSWD.as_ptr() as *const c_char,
    pw_uid: UserIdentifier::ROOT.as_usize() as uid_t,
    pw_gid: GroupIdentifier::ROOT.as_usize() as gid_t,
    pw_gecos: PW_GECOS.as_ptr() as *const c_char,
    pw_dir: ptr::addr_of!(PW_DIR) as *const c_char,
    pw_shell: PW_SHELL.as_ptr() as *const c_char,
}));

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
/// Returns the synthetic `root` password database entry. Lookups for other user IDs return a null
/// pointer. The home directory (`pw_dir`) is taken from the `HOME` environment variable when set,
/// and falls back to `"/"` otherwise.
///
/// # Parameters
///
/// - `uid`: The user ID to look up.
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
    if uid != UserIdentifier::ROOT.as_usize() as uid_t {
        return ptr::null_mut();
    }

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

    PW_ENTRY.0.get()
}

//==================================================================================================
// getpwnam()
//==================================================================================================

///
/// # Description
///
/// Returns the synthetic `root` password database entry. Other names return a null pointer.
///
/// # Parameters
///
/// - `name`: The login name to look up.
///
/// # Returns
///
/// A pointer to a statically-allocated [`passwd`] entry describing the `root` user. Per POSIX, the
/// storage may be overwritten by subsequent calls.
///
/// # Safety
///
/// `name` must point to a readable NUL-terminated string. This function returns a pointer to static
/// storage that must not be freed by the caller and may be overwritten by subsequent calls. It is
/// not thread-safe: callers must ensure no other thread calls `getpwnam()` or `getpwuid()` while the
/// returned pointer is still in use.
///
#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
#[trace_libcall]
pub unsafe extern "C" fn getpwnam(name: *const c_char) -> *mut passwd {
    if name.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: the caller provides a valid NUL-terminated login name.
    if unsafe { CStr::from_ptr(name) }.to_bytes() != b"root" {
        return ptr::null_mut();
    }

    let root_uid: uid_t = UserIdentifier::ROOT.as_usize() as uid_t;
    unsafe { getpwuid(root_uid) }
}
