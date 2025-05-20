// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::fcntl::LibcAtFlags;
use ::alloc::ffi::CString;
use ::core::{
    ffi,
    ffi::CStr,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::syscall::{
    ffi::c_int,
    limits,
    message::MessagePartitioner,
    sys::types::{
        off_t,
        size_t,
        ssize_t,
    },
    unistd,
    unistd::message::{
        ChangeDirectoryRequest,
        ChangeDirectoryResponse,
        CloseRequest,
        CloseResponse,
        FileAccessAtRequest,
        FileAccessAtResponse,
        FileChdirRequest,
        FileChdirResponse,
        FileChownRequest,
        FileChownResponse,
        FileDataSyncRequest,
        FileDataSyncResponse,
        FileSyncRequest,
        FileSyncResponse,
        FileTruncateRequest,
        FileTruncateResponse,
        GetCurrentWorkingDirectoryResponse,
        GetIdsRequest,
        GetIdsResponse,
        LinkAtRequest,
        LinkAtResponse,
        PartialReadRequest,
        PartialReadResponse,
        PartialWriteRequest,
        PartialWriteResponse,
        PipeResponse,
        ReadRequest,
        ReadResponse,
        SeekRequest,
        SeekResponse,
        WriteRequest,
        WriteResponse,
    },
};

//==================================================================================================
// do_chdir
//==================================================================================================

pub fn do_chdir(pid: ProcessIdentifier, request: ChangeDirectoryRequest) -> Vec<Message> {
    trace!("do_chdir(): pid={pid:?}, request={request:?}");

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(error) => {
            error!("do_chdir(): invalid path (error={error:?})");
            return vec![crate::build_error(pid, ErrorCode::InvalidArgument)];
        },
    };

    debug!("libc::chdir(): path={path:?}");
    match unsafe { libc::chdir(path.as_ptr()) } {
        0 => {
            debug!("do_chdir(): chdir() succeeded");
            vec![ChangeDirectoryResponse::build(pid)]
        },
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::chdir(): errno={errno:?}");
            vec![crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )]
        },
    }
}

//==================================================================================================
// do_close
//==================================================================================================

pub fn do_close(pid: ProcessIdentifier, request: CloseRequest) -> Message {
    trace!("close(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::close(): fd={fd:?}");
    match unsafe { libc::close(fd) } {
        ret if ret == 0 => CloseResponse::build(pid, ret),
        _ => crate::build_error(pid, ErrorCode::InvalidArgument),
    }
}

//==================================================================================================
// do_faccessat
//==================================================================================================

pub fn do_faccessat(pid: ProcessIdentifier, request: FileAccessAtRequest) -> Vec<Message> {
    trace!("faccessat(): pid={request:?}, request={pid:?}");

    let dirfd: c_int = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);
    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidArgument)],
    };
    let amode: i32 = request.mode;
    let flag: LibcAtFlags = LibcAtFlags::from(request.flag);

    debug!(
        "libc::faccessat(): dirfd={:?}, path={path:?}, mode={amode:?}, flag={:?}",
        dirfd.inner(),
        flag.inner()
    );
    match unsafe { libc::faccessat(dirfd.inner(), path.as_ptr(), amode, flag.inner()) } {
        0 => vec![FileAccessAtResponse::build(pid)],
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::faccessat(): errno={errno:?}");
            vec![crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )]
        },
    }
}

//==================================================================================================
// do_fdatasync
//==================================================================================================

pub fn do_fdatasync(pid: ProcessIdentifier, request: FileDataSyncRequest) -> Message {
    trace!("fdatasync(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fdatasync(): fd={fd:?}");
    match unsafe { libc::fdatasync(fd) } {
        ret if ret == 0 => FileDataSyncResponse::build(pid, ret),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::fdatasync(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_getids
//==================================================================================================

pub fn do_getids(pid: ProcessIdentifier, _request: GetIdsRequest) -> Message {
    trace!("getids(): pid={pid:?}");

    // Get user ID.
    let uid: libc::uid_t = unsafe { libc::getuid() };
    debug!("libc::getuid(): uid={uid:?}");

    // Get effective user ID.
    let euid: libc::uid_t = unsafe { libc::geteuid() };
    debug!("libc::geteuid(): euid={euid:?}");

    // Get group ID.
    let gid: libc::gid_t = unsafe { libc::getgid() };
    debug!("libc::getgid(): gid={gid:?}");

    // Get effective group ID.
    let egid: libc::gid_t = unsafe { libc::getegid() };
    debug!("libc::getegid(): egid={egid:?}");

    // Build response.
    GetIdsResponse::build(pid, uid, gid, euid, egid)
}

//==================================================================================================
// do_getcwd
//==================================================================================================

pub fn do_getcwd(pid: ProcessIdentifier) -> Vec<Message> {
    trace!("getcwd(): pid={pid:?}");

    let mut buf: Vec<u8> = Vec::with_capacity(limits::PATH_MAX as libc::size_t);

    // Get current working directory and check for errors.
    debug!("libc::getcwd(): buf={:p}, size={:?}", buf.as_mut_ptr(), buf.capacity());
    if unsafe { !libc::getcwd(buf.as_mut_ptr() as *mut libc::c_char, buf.capacity()).is_null() } {
        // Build response and check for errors.
        let response: GetCurrentWorkingDirectoryResponse =
            match unsafe { CStr::from_ptr(buf.as_ptr() as *const libc::c_char).to_str() } {
                // Success.
                Ok(cwd) => {
                    debug!("libc::getcwd(): cwd={cwd:?}");
                    match GetCurrentWorkingDirectoryResponse::new(cwd) {
                        Ok(response) => response,
                        Err(error) => {
                            warn!("do_getcwd(): {error:?}");
                            return vec![crate::build_error(pid, error.code)];
                        },
                    }
                },
                // Failure.
                Err(error) => {
                    error!("do_getcwd(): invalid path (error={error:?})");
                    return vec![crate::build_error(pid, ErrorCode::InvalidArgument)];
                },
            };

        // Build response parts and check for errors.
        match response.into_parts(pid) {
            Ok(messages) => messages,
            Err(error) => {
                warn!("do_getcwd(): {error:?}");
                vec![crate::build_error(pid, error.code)]
            },
        }
    } else {
        let errno: i32 = unsafe { *libc::__errno_location() };
        debug!("libc::getcwd(): errno={pid:?}");
        let error: ErrorCode =
            ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
        vec![crate::build_error(pid, error)]
    }
}

//==================================================================================================
// do_fsync
//==================================================================================================

pub fn do_fsync(pid: ProcessIdentifier, request: FileSyncRequest) -> Message {
    trace!("fsync(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fsync(): fd={fd:?}");
    match unsafe { libc::fsync(fd) } {
        ret if ret == 0 => FileSyncResponse::build(pid, ret),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::fsync(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_lseek
//==================================================================================================

pub fn do_lseek(pid: ProcessIdentifier, request: SeekRequest) -> Message {
    trace!("lseek(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let offset: i64 = request.offset;
    let whence: LibcSeek = match LibcSeek::try_from(request.whence) {
        Ok(whence) => whence,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    debug!("libc::lseek(): fd={:?}, offset={:?}, whence={:?}", fd, offset, whence.inner());
    match unsafe { libc::lseek(fd, offset, whence.inner()) } {
        ret if ret >= 0 => SeekResponse::build(pid, ret),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::lseek(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_ftruncate
//==================================================================================================

pub fn do_ftruncate(pid: ProcessIdentifier, request: FileTruncateRequest) -> Message {
    trace!("ftruncate(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let length: off_t = request.length;

    debug!("libc::ftruncate(): fd={fd:?}, length={length:?}");
    match unsafe { libc::ftruncate(fd, length) } {
        ret if ret == 0 => FileTruncateResponse::build(pid, ret),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::ftruncate(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_write
//==================================================================================================

pub fn do_write(pid: ProcessIdentifier, request: WriteRequest) -> Message {
    trace!("write(): pid={pid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > WriteRequest::BUFFER_SIZE as size_t {
        return crate::build_error(pid, ErrorCode::InvalidArgument);
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let buffer: &[u8] = &request.buffer[..count];

    debug!("libc::write(): fd={fd:?}, buffer={buffer:?}");
    match unsafe { libc::write(fd, buffer.as_ptr() as *const _, count) } {
        ret if ret >= 0 => WriteResponse::build(pid, ret as i32),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::write(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_read
//==================================================================================================

pub fn do_read(pid: ProcessIdentifier, request: ReadRequest) -> Message {
    trace!("read(): pid={pid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > ReadResponse::BUFFER_SIZE as size_t {
        return crate::build_error(pid, ErrorCode::InvalidArgument);
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let mut buffer: [u8; ReadResponse::BUFFER_SIZE] = [0; ReadResponse::BUFFER_SIZE];

    debug!("libc::read(): fd={fd:?}, buffer={buffer:?}");
    match unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut _, count) } {
        ret if ret >= 0 => ReadResponse::build(pid, ret as i32, buffer),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::read(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_pwrite
//==================================================================================================

pub fn do_pwrite(pid: ProcessIdentifier, request: PartialWriteRequest) -> Message {
    trace!("pwrite(): pid={pid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > PartialWriteRequest::BUFFER_SIZE as size_t {
        return crate::build_error(pid, ErrorCode::InvalidArgument);
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;
    let offset: off_t = request.offset;

    let buffer: &[u8] = &request.buffer[..count];

    debug!("libc::pwrite(): fd={fd:?}, count={count:?}, offset={offset:?}, buffer={buffer:?}",);
    match unsafe { libc::pwrite(fd, buffer.as_ptr() as *const _, count, offset) } {
        ret if ret >= 0 => PartialWriteResponse::build(pid, ret as ssize_t),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::pwrite(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_pread
//==================================================================================================

pub fn do_pread(pid: ProcessIdentifier, request: PartialReadRequest) -> Message {
    trace!("pread(): pid={pid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > PartialReadResponse::BUFFER_SIZE as size_t {
        return crate::build_error(pid, ErrorCode::InvalidArgument);
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;
    let offset: off_t = request.offset;

    let mut buffer: [u8; PartialReadResponse::BUFFER_SIZE] = [0; PartialReadResponse::BUFFER_SIZE];

    debug!("libc::pread(): fd={fd:?}, count={count:?}, offset={offset:?}, buffer={buffer:?}",);
    match unsafe { libc::pread(fd, buffer.as_mut_ptr() as *mut _, count, offset) } {
        ret if ret >= 0 => PartialReadResponse::build(pid, ret as ssize_t, buffer),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::pread(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
    }
}

//==================================================================================================
// do_linkat
//==================================================================================================

pub fn do_linkat(pid: ProcessIdentifier, request: LinkAtRequest) -> Vec<Message> {
    trace!("linkat(): pid={pid:?}, request={request:?}");

    let olddirfd: i32 = request.olddirfd;
    let oldpath: CString = match CString::new(request.oldpath.as_str()) {
        Ok(oldpath) => oldpath,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidArgument)],
    };
    let newdirfd: i32 = request.newdirfd;
    let newpath: CString = match CString::new(request.newpath.as_str()) {
        Ok(newpath) => newpath,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidArgument)],
    };
    let flags: i32 = request.flags;

    debug!(
        "libc::linkat(): olddirfd={olddirfd:?}, oldpath={oldpath:?}, newdirfd={newdirfd:?}, \
         newpath={newpath:?}, flags={flags:?}",
    );
    match unsafe { libc::linkat(olddirfd, oldpath.as_ptr(), newdirfd, newpath.as_ptr(), flags) } {
        ret if ret == 0 => vec![LinkAtResponse::build(pid, ret)],
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::linkat(): errno={errno:?}");
            vec![crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )]
        },
    }
}

//==================================================================================================
// do_fchdir
//==================================================================================================

/// Changes the current working directory.
pub fn do_fchdir(pid: ProcessIdentifier, request: FileChdirRequest) -> Message {
    trace!("fchdir(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fchdir(): fd={fd:?}");
    match unsafe { libc::fchdir(fd) } {
        0 => FileChdirResponse::build(pid),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
        ret => unreachable!("libc::fchdir() returned an invalid value ({:?})", ret),
    }
}

//==================================================================================================
// do_fchown
//==================================================================================================

pub fn do_fchown(pid: ProcessIdentifier, request: FileChownRequest) -> Message {
    trace!("fchown(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let owner: u32 = request.owner;
    let group: u32 = request.group;

    debug!("libc::fchown(): fd={fd:?}, owner={owner:?}, group={group:?}");
    match unsafe { libc::fchown(fd, owner, group) } {
        0 => FileChownResponse::build(pid),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
        ret => unreachable!("libc::fchown() returned an invalid value ({:?})", ret),
    }
}

//==================================================================================================
// do_pipe
//==================================================================================================

pub fn do_pipe(pid: ProcessIdentifier) -> Message {
    trace!("pipe(): pid={pid:?}");

    let mut fds: [i32; 2] = [0; 2];

    debug!("libc::pipe(): fds={fds:?}");
    match unsafe { libc::pipe(fds.as_mut_ptr()) } {
        0 => {
            let read_fd: i32 = fds[0];
            let write_fd: i32 = fds[1];

            debug!("pipe(): read_fd={read_fd:?}, write_fd={write_fd:?}");
            PipeResponse::build(pid, read_fd, write_fd)
        },
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::pipe(): errno={errno:?}");
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
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
