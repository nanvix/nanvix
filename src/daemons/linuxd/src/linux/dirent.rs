// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    syscalls::{
        SyscallAction,
        SyscallTable,
    },
};
use ::alloc::vec::Vec;
use ::core::{
    cmp,
    mem,
};
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::ffi::CStr;
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::syscall::{
    dirent::message::{
        GetDirectoryEntriesRequest,
        GetDirectoryEntriesResponse,
    },
    message::MessagePartitioner,
};
use sysapi::{
    dirent::{
        dirent_file_type::{
            DT_BLK,
            DT_CHR,
            DT_DIR,
            DT_FIFO,
            DT_LNK,
            DT_REG,
            DT_SOCK,
            DT_UNKNOWN,
        },
        posix_dent,
    },
    limits::XOPEN_NAME_MAX,
    sys_types::reclen_t,
};

//==================================================================================================
// linux_dirent
//==================================================================================================

///
/// # Description
///
/// A type representing a directory entry in Linux.
///
/// # Note
///
/// This type is not exposed to libc, so we need to provide them ourselves.
/// See https://www.man7.org/linux/man-pages/man2/getdents.2.html.
///
#[allow(non_camel_case_types)]
#[repr(C, packed)]
pub struct linux_dirent {
    /// File serial number.
    pub d_ino: libc::c_ulong,
    /// Filesystem-specific value with no specific meaning to user space.
    pub d_off: libc::off_t,
    /// Length of this entry.
    pub d_reclen: libc::c_ushort,
    // File name (including null terminator character).
    // Length is actually (d_reclen - 2 - offsetof(struct linux_dirent, d_name))
    pub d_name: [libc::c_char; 0],
    // Zero padding byte.
    // pub padding: libc::c_char,
    // File type (only since Linux 2.6.4); offset is (d_reclen - 1)
    // pub d_type: libc::c_uchar,
}

impl linux_dirent {
    /// Minimum size of a `linux_dirent` structure.
    const MIN_SIZE: usize = mem::size_of::<libc::c_ulong>() +  // d_ino
                   mem::size_of::<libc::off_t>() +  // d_off
                   mem::size_of::<libc::c_ushort>() +  // d_reclen
                   XOPEN_NAME_MAX+1; // d_name

    /// Size of `d_ino` field, used for static assertions.
    const _SIZE_OF_D_INO: usize = mem::size_of::<libc::c_ulong>();
    /// Size of `d_off` field, used for static assertions.
    const _SIZE_OF_D_OFF: usize = mem::size_of::<libc::off_t>();
    /// Size of `d_reclen` field, used for static assertions.
    const _SIZE_OF_D_RECLEN: usize = mem::size_of::<libc::c_ushort>();

    /// Offset of `d_ino` field, used for static assertions.
    const _OFFSET_OF_D_INO: usize = 0;
    /// Offset of `d_off` field, used for static assertions.
    const _OFFSET_OF_D_OFF: usize = Self::_OFFSET_OF_D_INO + Self::_SIZE_OF_D_INO;
    /// Offset of `d_reclen` field, used for static assertions.
    const _OFFSET_OF_D_RECLEN: usize = Self::_OFFSET_OF_D_OFF + Self::_SIZE_OF_D_OFF;
    /// Offset of `d_name` field, used for static assertions.
    const _OFFSET_OF_D_NAME: usize = Self::_OFFSET_OF_D_RECLEN + Self::_SIZE_OF_D_RECLEN;
}

//==================================================================================================
// do_getdents
//==================================================================================================

/// Handles a getdents() system call request.
pub fn do_getdents<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: GetDirectoryEntriesRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("do_getdents(): tid={tid:?}, request,count={:#x?}", { request.count });

    // Check if `request.count` is not valid.
    if request.count == 0 {
        error!("do_getdents(): invalid buffer count");
        return Ok(vec![crate::build_error(tid, ErrorCode::InvalidArgument)]);
    } else if request.count as usize > GetDirectoryEntriesRequest::MAX_ENTRIES {
        error!("do_getdents(): request is too large");
        return Ok(vec![crate::build_error(tid, ErrorCode::TooBig)]);
    }

    let bufsize: usize =
        (request.count as usize) * cmp::max(mem::size_of::<posix_dent>(), linux_dirent::MIN_SIZE);
    let mut rawbuf: Vec<u8> = vec![0; { bufsize } as usize];
    let mut buf: Vec<posix_dent> = Vec::new();

    // Get directory entries and check for errors.
    debug!("libc::getdents(): fd={}, buf={:#x?}, bufsize={}", { request.fd }, rawbuf.as_ptr(), {
        bufsize
    });
    match unsafe { handle_getdents(syscall_table, request.fd, rawbuf.as_mut_ptr(), bufsize) } {
        // Failed.
        -1 => {
            let errno = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_getdents(): worker thread interrupted while blocked on getdents()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::getdents(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_getdents(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            return Ok(vec![crate::build_error(tid, error)]);
        },
        // Success.
        n => {
            debug!("libc::getdents(): returned count={n}");

            let mut bpos: usize = 0;
            while bpos < n as usize {
                let dent: &linux_dirent =
                    unsafe { &*(rawbuf.as_ptr().add(bpos) as *const linux_dirent) };
                let d_reclen: usize = dent.d_reclen as usize;
                if bpos + d_reclen > n as usize {
                    // FIXME: should we rewind the file?
                    unimplemented!(
                        "do_getdents(): d_reclen exceeds buffer size, stopping iteration"
                    );
                }
                let d_type: libc::c_uchar =
                    unsafe { *rawbuf.get_unchecked(bpos + d_reclen - 1) as libc::c_uchar };
                let d_name_ptr: *const u8 =
                    unsafe { rawbuf.as_ptr().add(bpos + linux_dirent::_OFFSET_OF_D_NAME) };
                let d_name: &CStr = unsafe { CStr::from_ptr(d_name_ptr as *const libc::c_char) };
                debug!(
                    "libc::getdents(): d_ino={:#x}, d_off={:#x}, d_reclen={d_reclen}, \
                     d_type={d_type}, d_name={:?}",
                    { dent.d_ino },
                    { dent.d_off },
                    d_name,
                );

                let mut nanvix_dent: posix_dent = posix_dent {
                    d_ino: dent.d_ino,
                    d_reclen: mem::size_of::<posix_dent>() as reclen_t,
                    d_type: match d_type {
                        libc::DT_FIFO => DT_FIFO,
                        libc::DT_CHR => DT_CHR,
                        libc::DT_DIR => DT_DIR,
                        libc::DT_BLK => DT_BLK,
                        libc::DT_REG => DT_REG,
                        libc::DT_LNK => DT_LNK,
                        libc::DT_SOCK => DT_SOCK,
                        _ => DT_UNKNOWN,
                    },
                    ..Default::default()
                };
                let d_name: &[u8] = d_name.to_bytes_with_nul();
                let d_name_len: usize = d_name.len();
                nanvix_dent.d_name[..d_name_len].copy_from_slice(&d_name[..d_name_len]);
                buf.push(nanvix_dent);

                bpos += d_reclen;
            }
        },
    }

    // Build response and check for errors.
    let response: GetDirectoryEntriesResponse = GetDirectoryEntriesResponse::new(buf);
    match response.into_parts(tid, ::syscall::LINUXD, MessageType::Ikc) {
        Ok(messages) => Ok(messages),
        Err(error) => {
            warn!("do_getdents(): failed to build response (error={error:?})");
            Ok(vec![crate::build_error(tid, error.code)])
        },
    }
}

//==================================================================================================
// System Call Wrappers
//==================================================================================================

/// Handler for `getdents()` system call.
unsafe fn handle_getdents<T>(
    syscall_table: &SyscallTable<T>,
    fd: libc::c_int,
    dirp: *mut u8,
    count: libc::size_t,
) -> libc::c_long {
    match &syscall_table.getdents {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, fd, dirp, count)
        },
    }
}
