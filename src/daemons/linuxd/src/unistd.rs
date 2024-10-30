// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi;
use ::linuxd::{
    sys::types::{
        off_t,
        size_t,
        ssize_t,
    },
    unistd,
    unistd::message::{
        CloseRequest,
        CloseResponse,
        FileDataSyncRequest,
        FileDataSyncResponse,
        FileSyncRequest,
        FileSyncResponse,
        FileTruncateRequest,
        FileTruncateResponse,
        PartialWriteRequest,
        PartialWriteResponse,
        ReadRequest,
        ReadResponse,
        SeekRequest,
        SeekResponse,
        WriteRequest,
        WriteResponse,
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
// do_ftruncate
//==================================================================================================

pub fn do_ftruncate(pid: ProcessIdentifier, request: FileTruncateRequest) -> Message {
    trace!("ftruncate(): pid={:?}, request={:?}", pid, request);

    let fd: i32 = request.fd;
    let length: off_t = request.length;

    debug!("libc::ftruncate(): fd={:?}, length={:?}", fd, length);
    match unsafe { libc::ftruncate(fd, length) } {
        ret if ret == 0 => FileTruncateResponse::build(pid, ret),
        ret => crate::build_error(
            pid,
            ErrorCode::try_from(ret).unwrap_or_else(|_| panic!("invalid error code: {:?}", ret)),
        ),
    }
}

//==================================================================================================
// do_write
//==================================================================================================

pub fn do_write(pid: ProcessIdentifier, request: WriteRequest) -> Message {
    trace!("write(): pid={:?}, request={:?}", pid, request);

    // Check if count is invalid.
    if request.count > WriteRequest::BUFFER_SIZE as size_t {
        return crate::build_error(pid, ErrorCode::InvalidArgument);
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let buffer: &[u8] = &request.buffer[..count];

    debug!("libc::write(): fd={:?}, buffer={:?}", fd, buffer);
    match unsafe { libc::write(fd, buffer.as_ptr() as *const _, count) } {
        ret if ret >= 0 => WriteResponse::build(pid, ret as i32),
        ret => crate::build_error(
            pid,
            ErrorCode::try_from(ret as i32)
                .unwrap_or_else(|_| panic!("invalid error code: {:?}", ret)),
        ),
    }
}

//==================================================================================================
// do_read
//==================================================================================================

pub fn do_read(pid: ProcessIdentifier, request: ReadRequest) -> Message {
    trace!("read(): pid={:?}, request={:?}", pid, request);

    // Check if count is invalid.
    if (request.count < 0) || (request.count > ReadResponse::BUFFER_SIZE as i32) {
        return crate::build_error(pid, ErrorCode::InvalidArgument);
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let mut buffer: [u8; ReadResponse::BUFFER_SIZE] = [0; ReadResponse::BUFFER_SIZE];

    debug!("libc::read(): fd={:?}, buffer={:?}", fd, buffer);
    match unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut _, count) } {
        ret if ret >= 0 => ReadResponse::build(pid, ret as i32, buffer),
        ret => crate::build_error(
            pid,
            ErrorCode::try_from(ret as i32)
                .unwrap_or_else(|_| panic!("invalid error code: {:?}", ret)),
        ),
    }
}

//==================================================================================================
// do_pwrite
//==================================================================================================

pub fn do_pwrite(pid: ProcessIdentifier, request: PartialWriteRequest) -> Message {
    trace!("pwrite(): pid={:?}, request={:?}", pid, request);

    // Check if count is invalid.
    if request.count > PartialWriteRequest::BUFFER_SIZE as size_t {
        return crate::build_error(pid, ErrorCode::InvalidArgument);
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;
    let offset: off_t = request.offset;

    let buffer: &[u8] = &request.buffer[..count];

    debug!(
        "libc::pwrite(): fd={:?}, count={:?}, offset={:?}, buffer={:?}",
        fd, count, offset, buffer
    );
    match unsafe { libc::pwrite(fd, buffer.as_ptr() as *const _, count, offset) } {
        ret if ret >= 0 => PartialWriteResponse::build(pid, ret as ssize_t),
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
