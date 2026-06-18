// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::streams::FILE;
use ::core::sync::atomic::{
    AtomicU32,
    Ordering,
};
use ::sysapi::{
    errno::{
        __errno_location,
        EEXIST,
    },
    fcntl::{
        file_access_mode::O_RDWR,
        file_creation_flags::{
            O_CREAT,
            O_EXCL,
        },
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_types::mode_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a temporary file opened for update (`"w+b"`) under `/tmp`. The file is given a unique
/// name; unlike a full implementation it is not automatically removed on close, because the
/// standalone file system does not support unlinking an open file.
///
/// # Returns
///
/// A pointer to a [`FILE`] on success, or a null pointer on failure.
///
/// # Safety
///
/// This function is unsafe because it calls into the C runtime and dereferences raw pointers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/tmpfile.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn tmpfile() -> *mut FILE {
    extern "C" {
        fn open(path: *const c_char, flags: c_int, mode: mode_t) -> c_int;
        fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
        fn close(fd: c_int) -> c_int;
    }

    /// Process-wide counter used to build unique temporary file names.
    static COUNTER: AtomicU32 = AtomicU32::new(0);

    // Template: "/tmp/tmpfXXXXXXXX\0" (8 hex digits).
    let mut name: [u8; 18] = *b"/tmp/tmpf00000000\0";

    for _ in 0..64 {
        // Atomically reserve a unique id so concurrent callers neither race nor collide.
        let id: u32 = COUNTER.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        let mut v: u32 = id;
        let mut idx: usize = 16;
        while idx >= 9 {
            let digit: u8 = (v & 0xf) as u8;
            name[idx] = if digit < 10 {
                b'0' + digit
            } else {
                b'a' + (digit - 10)
            };
            v >>= 4;
            idx -= 1;
        }

        // SAFETY: name is a valid, null-terminated path; mode is valid.
        let fd: c_int =
            unsafe { open(name.as_ptr().cast::<c_char>(), O_RDWR | O_CREAT | O_EXCL, 0o600) };
        if fd >= 0 {
            // SAFETY: fd is a freshly opened, valid file descriptor.
            let stream: *mut FILE = unsafe { fdopen(fd, c"w+b".as_ptr()) };
            if stream.is_null() {
                // fdopen() failed after the file was created: close the descriptor so it is
                // not leaked, then give up (a retry cannot recover an allocation failure).
                // SAFETY: fd is a valid open descriptor.
                unsafe { close(fd) };
                return core::ptr::null_mut();
            }
            return stream;
        }

        // Only a name collision (EEXIST) is worth retrying with a fresh name. Any other error
        // (e.g. missing /tmp, permission denied) is non-transient and will never succeed, so
        // bail out immediately and let errno describe the failure.
        // SAFETY: __errno_location returns a valid pointer to the errno storage.
        if unsafe { *__errno_location() } != EEXIST {
            return core::ptr::null_mut();
        }
    }

    core::ptr::null_mut()
}
