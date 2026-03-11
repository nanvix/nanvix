// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use sysapi::sys_stat;
#[cfg(not(feature = "standalone"))]
use {
    crate::sys::stat::message::FileStatRequest,
    ::sys::{
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

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
#[allow(unreachable_code)]
pub fn fstat(fd: i32, buf: &mut sys_stat::stat) -> Result<(), Error> {
    // In standalone mode, forward operation to virtual file system (VFS).
    #[cfg(feature = "standalone")]
    {
        if ::nvx::vfs::fd::is_vfs_fd(fd) {
            return ::nvx::vfs::fd::vfs_fstat(fd, buf).map_err(|e| {
                let code: ::sys::error::ErrorCode = e.into();
                ::syslog::error!("fstat(): VFS fstat failed (fd={fd}, error={e})");
                Error::new(code, "vfs fstat failed")
            });
        }
        use ::sysapi::{
            sys_stat::{
                file_mode,
                file_type,
            },
            unistd::{
                STDERR_FILENO,
                STDIN_FILENO,
                STDOUT_FILENO,
            },
        };
        if fd == STDIN_FILENO || fd == STDOUT_FILENO || fd == STDERR_FILENO {
            // SAFETY: zeroes all bytes of `buf` before field assignment.
            unsafe {
                ::core::ptr::write_bytes(buf, 0, 1);
            }
            buf.st_mode = file_type::S_IFCHR | file_mode::S_IRUSR | file_mode::S_IWUSR;
            // Block size matches the page-sized granularity of push/pull kernel calls when
            // crossing the VM boundary.
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
        Err(Error::new(::sys::error::ErrorCode::BadFile, "fstat: invalid fd in standalone mode"))
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fstat_linuxd(fd, buf)
}

/// Forwards a `fstat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fstat_linuxd(fd: i32, buf: &mut sys_stat::stat) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;
    let message: Message = FileStatRequest::build(tid, fd);
    ::sys::kcall::ipc::send(&message)?;

    *buf = crate::sys::stat::syscall::fstatat_response()?;

    Ok(())
}
