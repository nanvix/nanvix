// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    dirent::DirectoryStream,
    errno::__errno_location,
};
use ::alloc::boxed::Box;
use ::core::{
    mem::ManuallyDrop,
    ptr,
};
use ::sys::error::ErrorCode;
use ::sysapi::dirent::dirent;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `readdir()` system call reads the next directory entry from the directory stream associated
/// with `dirp`. The function returns a pointer to a `dirent` structure representing the next
/// directory entry in the directory stream pointed to by `dirp`. It returns a null pointer upon
/// reaching the end of the directory stream.
///
/// The `dirent` structure is defined to include at least the following members:
/// - `d_ino`: File serial number (inode number)
/// - `d_name`: Null-terminated filename
///
/// The data returned by `readdir()` may be overwritten by subsequent calls to `readdir()` for
/// the same directory stream. It will not be overwritten by calls to `readdir()` for other
/// directory streams.
///
/// # Parameters
///
/// - `dirp`: Pointer to a `DirectoryStream` object that was returned by a previous call to `opendir()`.
///
/// # Returns
///
/// Upon successful completion, `readdir()` returns a pointer to a `dirent` structure. If the end
/// of the directory stream is encountered, a null pointer is returned and `errno` is not changed.
/// If an error occurs, a null pointer is returned and `errno` is set to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference a raw pointer (`dirp`).  It is safe to call
/// this function if and only if all the following conditions are met:
/// - `dirp` points to a valid `DirectoryStream` object that was previously returned by `opendir()`
/// - `dirp` has not been closed by a previous call to `closedir()`
/// - `dirp` remains valid for the duration of the function call
///
/// The returned pointer points to static data that may be overwritten by subsequent calls to
/// `readdir()` on the same directory stream. The caller should not attempt to free the returned
/// pointer, as it points to internally managed memory.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn readdir(dirp: *mut DirectoryStream) -> *mut dirent {
    // Check if directory stream is invalid.
    if dirp.is_null() {
        ::syslog::error!("readdir(): invalid directory stream (dirp={dirp:?})");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return ptr::null_mut();
    }

    let mut dirp: ManuallyDrop<Box<DirectoryStream>> = ManuallyDrop::new(Box::from_raw(dirp));

    // Read directory entry and check for errors.
    let direntp: *mut dirent = match crate::dirent::readdir(&mut dirp) {
        // End of directory.
        Ok(None) => ptr::null_mut(),
        // Directory entry read.
        Ok(Some(dirent)) => {
            let last_dirent: &mut dirent = dirp.last_dirent_as_mut();
            last_dirent.d_ino = dirent.d_ino;
            last_dirent.d_name.copy_from_slice(&dirent.d_name);
            last_dirent as *mut dirent
        },
        // Error.
        Err(error) => {
            ::syslog::error!(
                "readdir(): failed to read directory entry (dirp={:?}, error={:?})",
                dirp,
                error
            );
            *__errno_location() = error.code.get();
            ptr::null_mut()
        },
    };

    direntp
}
