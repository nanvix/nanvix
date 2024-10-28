// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi;
use ::linuxd::{
    unistd,
    unistd::message::{
        CloseRequest,
        CloseResponse,
        FileDataSyncRequest,
        FileDataSyncResponse,
        FileSyncRequest,
        FileSyncResponse,
        SeekRequest,
        SeekResponse,
    },
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// do_close
//==================================================================================================

pub fn do_close(pid: ProcessIdentifier, request: CloseRequest) -> Message {
    trace!("close(): pid={:?}, request={:?}", pid, request);

    let fd: i32 = request.fd;

    debug!("libc::close(): fd={:?}", fd);
    match unsafe { libc::close(fd) } {
        ret if ret == 0 => CloseResponse::build(pid, ret),
        _ => crate::build_error(pid, ErrorCode::InvalidArgument),
    }
}

//==================================================================================================
// do_fdatasync
//==================================================================================================

pub fn do_fdatasync(pid: ProcessIdentifier, request: FileDataSyncRequest) -> Message {
    trace!("fdatasync(): pid={:?}, request={:?}", pid, request);

    let fd: i32 = request.fd;

    debug!("libc::fdatasync(): fd={:?}", fd);
    match unsafe { libc::fdatasync(fd) } {
        ret if ret == 0 => FileDataSyncResponse::build(pid, ret),
        ret => crate::build_error(
            pid,
            ErrorCode::try_from(ret).unwrap_or_else(|_| panic!("invalid error code: {:?}", ret)),
        ),
    }
}

//==================================================================================================
// do_fsync
//==================================================================================================

pub fn do_fsync(pid: ProcessIdentifier, request: FileSyncRequest) -> Message {
    trace!("fsync(): pid={:?}, request={:?}", pid, request);

    let fd: i32 = request.fd;

    debug!("libc::fsync(): fd={:?}", fd);
    match unsafe { libc::fsync(fd) } {
        ret if ret == 0 => FileSyncResponse::build(pid, ret),
        ret => crate::build_error(
            pid,
            ErrorCode::try_from(ret).unwrap_or_else(|_| panic!("invalid error code: {:?}", ret)),
        ),
    }
}

//==================================================================================================
// do_lseek
//==================================================================================================

pub fn do_lseek(pid: ProcessIdentifier, request: SeekRequest) -> Message {
    trace!("lseek(): pid={:?}, request={:?}", pid, request);

    let fd: i32 = request.fd;
    let offset: i64 = request.offset;
    let whence: LibcSeek = match LibcSeek::try_from(request.whence) {
        Ok(whence) => whence,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    debug!("libc::lseek(): fd={:?}, offset={:?}, whence={:?}", fd, offset, whence.inner());
    match unsafe { libc::lseek(fd, offset, whence.inner()) } {
        ret if ret >= 0 => SeekResponse::build(pid, ret),
        ret => crate::build_error(
            pid,
            ErrorCode::try_from(ret as i32)
                .unwrap_or_else(|_| panic!("invalid error code: {:?}", ret)),
        ),
    }
}

//==================================================================================================

struct LibcSeek(ffi::c_int);

impl LibcSeek {
    fn inner(&self) -> ffi::c_int {
        self.0
    }
}

impl TryFrom<i32> for LibcSeek {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            unistd::SEEK_CUR => Ok(LibcSeek(libc::SEEK_CUR)),
            unistd::SEEK_END => Ok(LibcSeek(libc::SEEK_END)),
            unistd::SEEK_SET => Ok(LibcSeek(libc::SEEK_SET)),
            unistd::SEEK_HOLE => Ok(LibcSeek(libc::SEEK_HOLE)),
            unistd::SEEK_DATA => Ok(LibcSeek(libc::SEEK_DATA)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid whence")),
        }
    }
}
