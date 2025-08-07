// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    fcntl::LibcAtFlags,
};
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
    pm::ThreadIdentifier,
};
use ::sysapi::{
    ffi::c_int,
    limits::PATH_MAX,
    sys_types::{
        c_size_t,
        c_ssize_t,
        off_t,
    },
    unistd::file_seek::{
        SEEK_CUR,
        SEEK_DATA,
        SEEK_END,
        SEEK_HOLE,
        SEEK_SET,
    },
};
use ::syscall::{
    message::MessagePartitioner,
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

pub fn do_chdir(tid: ThreadIdentifier, request: ChangeDirectoryRequest) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("do_chdir(): tid={tid:?}, request={request:?}");

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(error) => {
            error!("do_chdir(): invalid path (error={error:?})");
            return Ok(vec![crate::build_error(tid, ErrorCode::InvalidArgument)]);
        },
    };

    debug!("libc::chdir(): path={path:?}");
    match unsafe { libc::chdir(path.as_ptr()) } {
        0 => {
            debug!("do_chdir(): chdir() succeeded");
            Ok(vec![ChangeDirectoryResponse::build(tid)])
        },
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::chdir(): errno={errno:?}");
            Ok(vec![crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )])
        },
    }
}

//==================================================================================================
// do_close
//==================================================================================================

pub fn do_close(tid: ThreadIdentifier, request: CloseRequest) -> Result<Message, WorkerThreadError> {
    trace!("close(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::close(): fd={fd:?}");
    match unsafe { libc::close(fd) } {
        ret if ret == 0 => Ok(CloseResponse::build(tid, ret)),
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::close(): errno={errno:?}");
            Ok(crate::build_error(tid, ErrorCode::InvalidArgument))
        }
    }
}

//==================================================================================================
// do_faccessat
//==================================================================================================

pub fn do_faccessat(tid: ThreadIdentifier, request: FileAccessAtRequest) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("faccessat(): tid={request:?}, request={tid:?}");

    let dirfd: c_int = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);
    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidArgument)]),
    };
    let amode: i32 = request.mode;
    let flag: LibcAtFlags = LibcAtFlags::from(request.flag);

    debug!(
        "libc::faccessat(): dirfd={:?}, path={path:?}, mode={amode:?}, flag={:?}",
        dirfd.inner(),
        flag.inner()
    );
    match unsafe { libc::faccessat(dirfd.inner(), path.as_ptr(), amode, flag.inner()) } {
        0 => Ok(vec![FileAccessAtResponse::build(tid)]),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::faccessat(): errno={errno:?}");
            Ok(vec![crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )])
        },
    }
}

//==================================================================================================
// do_fdatasync
//==================================================================================================

pub fn do_fdatasync(tid: ThreadIdentifier, request: FileDataSyncRequest) -> Result<Message, WorkerThreadError> {
    trace!("fdatasync(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fdatasync(): fd={fd:?}");
    match unsafe { libc::fdatasync(fd) } {
        ret if ret == 0 => Ok(FileDataSyncResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::fdatasync(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_getids
//==================================================================================================

pub fn do_getids(tid: ThreadIdentifier, _request: GetIdsRequest) -> Result<Message, WorkerThreadError> {
    trace!("getids(): tid={tid:?}");

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
    Ok(GetIdsResponse::build(tid, uid, gid, euid, egid))
}

//==================================================================================================
// do_getcwd
//==================================================================================================

pub fn do_getcwd(tid: ThreadIdentifier) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("getcwd(): tid={tid:?}");

    let mut buf: Vec<u8> = Vec::with_capacity(PATH_MAX as libc::size_t);

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
                            return Ok(vec![crate::build_error(tid, error.code)]);
                        },
                    }
                },
                // Failure.
                Err(error) => {
                    error!("do_getcwd(): invalid path (error={error:?})");
                    return Ok(vec![crate::build_error(tid, ErrorCode::InvalidArgument)]);
                },
            };

        // Build response parts and check for errors.
        match response.into_parts(tid) {
            Ok(messages) => Ok(messages),
            Err(error) => {
                warn!("do_getcwd(): {error:?}");
                Ok(vec![crate::build_error(tid, error.code)])
            },
        }
    } else {
        let errno: i32 = unsafe { *libc::__errno_location() };

        // Check if the thread has been interrupted.
        if errno == libc::EINTR {
            return Err(WorkerThreadError::Interrupted);
        }

        debug!("libc::getcwd(): errno={tid:?}");
        let error: ErrorCode =
            ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
        Ok(vec![crate::build_error(tid, error)])
    }
}

//==================================================================================================
// do_fsync
//==================================================================================================

pub fn do_fsync(tid: ThreadIdentifier, request: FileSyncRequest) -> Result<Message, WorkerThreadError> {
    trace!("fsync(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fsync(): fd={fd:?}");
    match unsafe { libc::fsync(fd) } {
        ret if ret == 0 => Ok(FileSyncResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::fsync(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_lseek
//==================================================================================================

pub fn do_lseek(tid: ThreadIdentifier, request: SeekRequest) -> Result<Message, WorkerThreadError> {
    trace!("lseek(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let offset: i64 = request.offset;
    let whence: LibcSeek = match LibcSeek::try_from(request.whence) {
        Ok(whence) => whence,
        Err(_) => return Ok(crate::build_error(tid, ErrorCode::InvalidMessage)),
    };

    debug!("libc::lseek(): fd={:?}, offset={:?}, whence={:?}", fd, offset, whence.inner());
    match unsafe { libc::lseek(fd, offset, whence.inner()) } {
        ret if ret >= 0 => Ok(SeekResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::lseek(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_ftruncate
//==================================================================================================

pub fn do_ftruncate(tid: ThreadIdentifier, request: FileTruncateRequest) -> Result<Message, WorkerThreadError> {
    trace!("ftruncate(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let length: off_t = request.length;

    debug!("libc::ftruncate(): fd={fd:?}, length={length:?}");
    match unsafe { libc::ftruncate(fd, length) } {
        ret if ret == 0 => Ok(FileTruncateResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::ftruncate(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_write
//==================================================================================================

pub fn do_write(tid: ThreadIdentifier, request: WriteRequest) -> Result<Message, WorkerThreadError> {
    trace!("write(): tid={tid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > WriteRequest::BUFFER_SIZE as c_size_t {
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let buffer: &[u8] = &request.buffer[..count];

    debug!("libc::write(): fd={fd:?}, buffer={buffer:?}");
    match unsafe { libc::write(fd, buffer.as_ptr() as *const _, count) } {
        ret if ret >= 0 => Ok(WriteResponse::build(tid, ret as i32)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::write(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_read
//==================================================================================================

pub fn do_read(tid: ThreadIdentifier, request: ReadRequest) -> Result<Message, WorkerThreadError> {
    trace!("read(): tid={tid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > ReadResponse::BUFFER_SIZE as c_size_t {
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let mut buffer: [u8; ReadResponse::BUFFER_SIZE] = [0; ReadResponse::BUFFER_SIZE];

    debug!("libc::read(): fd={fd:?}, buffer={buffer:?}");
    match unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut _, count) } {
        ret if ret >= 0 => Ok(ReadResponse::build(tid, ret as i32, buffer)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::read(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_pwrite
//==================================================================================================

pub fn do_pwrite(tid: ThreadIdentifier, request: PartialWriteRequest) -> Result<Message, WorkerThreadError> {
    trace!("pwrite(): tid={tid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > PartialWriteRequest::BUFFER_SIZE as c_size_t {
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;
    let offset: off_t = request.offset;

    let buffer: &[u8] = &request.buffer[..count];

    debug!("libc::pwrite(): fd={fd:?}, count={count:?}, offset={offset:?}, buffer={buffer:?}",);
    match unsafe { libc::pwrite(fd, buffer.as_ptr() as *const _, count, offset) } {
        ret if ret >= 0 => Ok(PartialWriteResponse::build(tid, ret as c_ssize_t)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::pwrite(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_pread
//==================================================================================================

pub fn do_pread(tid: ThreadIdentifier, request: PartialReadRequest) -> Result<Message, WorkerThreadError> {
    trace!("pread(): tid={tid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > PartialReadResponse::BUFFER_SIZE as c_size_t {
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;
    let offset: off_t = request.offset;

    let mut buffer: [u8; PartialReadResponse::BUFFER_SIZE] = [0; PartialReadResponse::BUFFER_SIZE];

    debug!("libc::pread(): fd={fd:?}, count={count:?}, offset={offset:?}, buffer={buffer:?}",);
    match unsafe { libc::pread(fd, buffer.as_mut_ptr() as *mut _, count, offset) } {
        ret if ret >= 0 => Ok(PartialReadResponse::build(tid, ret as c_ssize_t, buffer)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::pread(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
    }
}

//==================================================================================================
// do_linkat
//==================================================================================================

pub fn do_linkat(tid: ThreadIdentifier, request: LinkAtRequest) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("linkat(): tid={tid:?}, request={request:?}");

    let olddirfd: i32 = request.olddirfd;
    let oldpath: CString = match CString::new(request.oldpath.as_str()) {
        Ok(oldpath) => oldpath,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidArgument)]),
    };
    let newdirfd: i32 = request.newdirfd;
    let newpath: CString = match CString::new(request.newpath.as_str()) {
        Ok(newpath) => newpath,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidArgument)]),
    };
    let flags: i32 = request.flags;

    debug!(
        "libc::linkat(): olddirfd={olddirfd:?}, oldpath={oldpath:?}, newdirfd={newdirfd:?}, \
         newpath={newpath:?}, flags={flags:?}",
    );
    match unsafe { libc::linkat(olddirfd, oldpath.as_ptr(), newdirfd, newpath.as_ptr(), flags) } {
        ret if ret == 0 => Ok(vec![LinkAtResponse::build(tid, ret)]),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::linkat(): errno={errno:?}");
            Ok(vec![crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )])
        },
    }
}

//==================================================================================================
// do_fchdir
//==================================================================================================

/// Changes the current working directory.
pub fn do_fchdir(tid: ThreadIdentifier, request: FileChdirRequest) -> Result<Message, WorkerThreadError> {
    trace!("fchdir(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fchdir(): fd={fd:?}");
    match unsafe { libc::fchdir(fd) } {
        0 => Ok(FileChdirResponse::build(tid)),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
        ret => unreachable!("libc::fchdir() returned an invalid value ({:?})", ret),
    }
}

//==================================================================================================
// do_fchown
//==================================================================================================

pub fn do_fchown(tid: ThreadIdentifier, request: FileChownRequest) -> Result<Message, WorkerThreadError> {
    trace!("fchown(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let owner: u32 = request.owner;
    let group: u32 = request.group;

    debug!("libc::fchown(): fd={fd:?}, owner={owner:?}, group={group:?}");
    match unsafe { libc::fchown(fd, owner, group) } {
        0 => Ok(FileChownResponse::build(tid)),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
        ret => unreachable!("libc::fchown() returned an invalid value ({:?})", ret),
    }
}

//==================================================================================================
// do_pipe
//==================================================================================================

pub fn do_pipe(tid: ThreadIdentifier) -> Result<Message, WorkerThreadError> {
    trace!("pipe(): tid={tid:?}");

    let mut fds: [i32; 2] = [0; 2];

    debug!("libc::pipe(): fds={fds:?}");
    match unsafe { libc::pipe(fds.as_mut_ptr()) } {
        0 => {
            let read_fd: i32 = fds[0];
            let write_fd: i32 = fds[1];

            debug!("pipe(): read_fd={read_fd:?}, write_fd={write_fd:?}");
            Ok(PipeResponse::build(tid, read_fd, write_fd))
        },
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::pipe(): errno={errno:?}");
            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
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
            SEEK_CUR => Ok(LibcSeek(libc::SEEK_CUR)),
            SEEK_END => Ok(LibcSeek(libc::SEEK_END)),
            SEEK_SET => Ok(LibcSeek(libc::SEEK_SET)),
            SEEK_HOLE => Ok(LibcSeek(libc::SEEK_HOLE)),
            SEEK_DATA => Ok(LibcSeek(libc::SEEK_DATA)),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid whence")),
        }
    }
}
