// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Commits filesystem caches to stable storage. Nanvix does not track per-filesystem state, so this
/// delegates to the global [`sync()`] and reports success.
///
/// # Parameters
///
/// - `fd`: File descriptor referring to a file on the filesystem to synchronize (ignored).
///
/// # Returns
///
/// `0` on success.
///
/// # Safety
///
/// This function is unsafe because it invokes the C `sync()` entry point.
///
/// # References
///
/// - <https://man7.org/linux/man-pages/man2/sync.2.html>
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syncfs(_fd: c_int) -> c_int {
    extern "C" {
        fn sync();
    }

    unsafe {
        sync();
    }
    0
}
