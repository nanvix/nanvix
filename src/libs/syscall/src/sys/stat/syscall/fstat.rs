// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::stat::message::FileStatRequest;
#[cfg(feature = "standalone")]
use ::sys::error::ErrorCode;
use ::sys::{
    error::Error,
    ipc::Message,
    pm::ThreadIdentifier,
};
use sysapi::sys_stat;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `stat()` system call obtains information about a file.
///
/// # Parameters
///
/// - `fd`: File descriptor of the file.
/// - `buf`: Buffer to store file information.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Upon failure, an error is returned
/// instead.
///
pub fn fstat(fd: i32, buf: &mut sys_stat::stat) -> Result<(), Error> {
    ::syslog::trace!("fstat(): fd={:?}", fd);

    // In standalone mode, synthesize a character-device stat for stdio fds so that
    // common libc patterns (isatty, buffering heuristics) continue to work.
    #[cfg(feature = "standalone")]
    {
        use ::sysapi::unistd::{
            STDERR_FILENO,
            STDIN_FILENO,
            STDOUT_FILENO,
        };
        if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
            use ::sysapi::sys_stat::{
                file_mode,
                file_type,
            };
            // SAFETY: zeroes all bytes of `buf` before field assignment.
            unsafe {
                ::core::ptr::write_bytes(buf, 0, 1);
            }
            buf.st_mode = file_type::S_IFCHR | file_mode::S_IRUSR | file_mode::S_IWUSR;
            // Block size matches the page-sized granularity of push/pull kernel calls.
            buf.st_blksize = ::arch::mem::PAGE_SIZE as i64;
            // Timestamp set to Unix epoch (1970-01-01T00:00:00 UTC).
            let ts = ::sysapi::time::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            buf.st_atim = ts;
            buf.st_mtim = ts;
            buf.st_ctim = ts;
            return Ok(());
        }

        // Only VFS file descriptors should be routed to vfsd.
        if !crate::is_vfs_fd(fd) {
            ::syslog::warn!("fstat(): bad file descriptor fd={fd} in standalone mode");
            return Err(Error::new(
                ErrorCode::BadFile,
                "fstat: fd is not a VFS fd in standalone mode",
            ));
        }
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::__kcall_gettid()?;
    let message: Message =
        FileStatRequest::build(tid, fd, crate::VFS_DESTINATION, crate::VFS_MESSAGE_TYPE);
    ::sys::kcall::ipc::__kcall_send(&message)?;

    *buf = crate::sys::stat::syscall::fstatat_response()?;

    Ok(())
}
