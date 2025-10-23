// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::WorkerThreadError,
    fcntl::LibcAtFlags,
    syscalls::{
        SystemCallAction,
        SystemCallRouteTable,
    },
};
use ::alloc::ffi::CString;
use ::core::{
    ffi,
    ffi::CStr,
};
use ::std::sync::Arc;
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
use ::syslog::{
    debug,
    error,
    trace,
    warn,
};

//==================================================================================================
// do_chdir
//==================================================================================================

pub fn do_chdir(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: ChangeDirectoryRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("do_chdir(): tid={tid:?}, request={request:?}");

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(error) => {
            error!("do_chdir(): invalid path (error={error:?})");
            return Ok(vec![crate::build_error(tid, ErrorCode::InvalidArgument)]);
        },
    };

    debug!("libc::chdir(): path={path:?}");
    match unsafe { handle_chdir(&syscall_table, path.as_ptr()) } {
        0 => {
            debug!("do_chdir(): chdir() succeeded");
            Ok(vec![ChangeDirectoryResponse::build(tid)])
        },
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_chdir(): worker thread interrupted while blocked on chdir()");
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

pub fn do_close(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: CloseRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("close(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::close(): fd={fd:?}");
    match unsafe { handle_close(&syscall_table, fd) } {
        ret if ret == 0 => Ok(CloseResponse::build(tid, ret)),
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_close(): worker thread interrupted while blocked on close()");
                return Err(WorkerThreadError::Interrupted);
            }

            debug!("libc::close(): errno={errno:?}");
            Ok(crate::build_error(tid, ErrorCode::InvalidArgument))
        },
    }
}

//==================================================================================================
// do_faccessat
//==================================================================================================

pub fn do_faccessat(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: FileAccessAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
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
    match unsafe {
        handle_faccessat(&syscall_table, dirfd.inner(), path.as_ptr(), amode, flag.inner())
    } {
        0 => Ok(vec![FileAccessAtResponse::build(tid)]),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_faccessat(): worker thread interrupted while blocked on faccessat()");
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

pub fn do_fdatasync(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: FileDataSyncRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("fdatasync(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fdatasync(): fd={fd:?}");
    match unsafe { handle_fdatasync(&syscall_table, fd) } {
        ret if ret == 0 => Ok(FileDataSyncResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fdatasync(): worker thread interrupted while blocked on fdatasync()");
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

pub fn do_getids(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    _request: GetIdsRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("getids(): tid={tid:?}");

    // Get user ID.
    let uid: libc::uid_t = unsafe { handle_getuid(&syscall_table) };
    debug!("libc::getuid(): uid={uid:?}");

    // Get effective user ID.
    let euid: libc::uid_t = unsafe { handle_geteuid(&syscall_table) };
    debug!("libc::geteuid(): euid={euid:?}");

    // Get group ID.
    let gid: libc::gid_t = unsafe { handle_getgid(&syscall_table) };
    debug!("libc::getgid(): gid={gid:?}");

    // Get effective group ID.
    let egid: libc::gid_t = unsafe { handle_getegid(&syscall_table) };
    debug!("libc::getegid(): egid={egid:?}");

    // Build response.
    Ok(GetIdsResponse::build(tid, uid, gid, euid, egid))
}

//==================================================================================================
// do_getcwd
//==================================================================================================

pub fn do_getcwd(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("getcwd(): tid={tid:?}");

    let mut buf: Vec<u8> = Vec::with_capacity(PATH_MAX as libc::size_t);

    // Get current working directory and check for errors.
    debug!("libc::getcwd(): buf={:p}, size={:?}", buf.as_mut_ptr(), buf.capacity());
    if unsafe {
        !handle_getcwd(&syscall_table, buf.as_mut_ptr() as *mut libc::c_char, buf.capacity())
            .is_null()
    } {
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
            error!("do_getcwd(): worker thread interrupted while blocked on getcwd()");
            return Err(WorkerThreadError::Interrupted);
        }

        error!("libc::getcwd(): errno={errno:?}");
        let error: ErrorCode = match ErrorCode::try_from(errno) {
            Ok(error) => error,
            Err(_) => {
                let reason: &str = "unknown error code";
                warn!("do_getcwd(): {reason} (errno={errno:?})");
                ErrorCode::ValueOutOfRange
            },
        };
        Ok(vec![crate::build_error(tid, error)])
    }
}

//==================================================================================================
// do_fsync
//==================================================================================================

pub fn do_fsync(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: FileSyncRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("fsync(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fsync(): fd={fd:?}");
    match unsafe { handle_fsync(&syscall_table, fd) } {
        ret if ret == 0 => Ok(FileSyncResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fsync(): worker thread interrupted while blocked on fsync()");
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

pub fn do_lseek(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: SeekRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("lseek(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let offset: i64 = request.offset;
    let whence: LibcSeek = match LibcSeek::try_from(request.whence) {
        Ok(whence) => whence,
        Err(_) => return Ok(crate::build_error(tid, ErrorCode::InvalidMessage)),
    };

    debug!("libc::lseek(): fd={:?}, offset={:?}, whence={:?}", fd, offset, whence.inner());
    match unsafe { handle_lseek(&syscall_table, fd, offset, whence.inner()) } {
        ret if ret >= 0 => Ok(SeekResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_lseek(): worker thread interrupted while blocked on lseek()");
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

pub fn do_ftruncate(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: FileTruncateRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("ftruncate(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let length: off_t = request.length;

    debug!("libc::ftruncate(): fd={fd:?}, length={length:?}");
    match unsafe { handle_ftruncate(&syscall_table, fd, length) } {
        ret if ret == 0 => Ok(FileTruncateResponse::build(tid, ret)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_ftruncate(): worker thread interrupted while blocked on ftruncate()");
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

pub fn do_write(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: WriteRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("write(): tid={tid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > WriteRequest::BUFFER_SIZE as c_size_t {
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let buffer: &[u8] = &request.buffer[..count];

    debug!("libc::write(): fd={fd:?}, buffer={buffer:?}");
    match unsafe { handle_write(&syscall_table, fd, buffer.as_ptr() as *const _, count) } {
        ret if ret >= 0 => Ok(WriteResponse::build(tid, ret as i32)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_write(): worker thread interrupted while blocked on write()");
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

pub fn do_read(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: ReadRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("read(): tid={tid:?}, request={request:?}");

    // Check if count is invalid.
    if request.count > ReadResponse::BUFFER_SIZE as c_size_t {
        return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
    }
    let fd: i32 = request.fd;
    let count: usize = request.count as usize;

    let mut buffer: [u8; ReadResponse::BUFFER_SIZE] = [0; ReadResponse::BUFFER_SIZE];

    debug!("libc::read(): fd={fd:?}, buffer={buffer:?}");
    match unsafe { handle_read(&syscall_table, fd, buffer.as_mut_ptr() as *mut _, count) } {
        ret if ret >= 0 => Ok(ReadResponse::build(tid, ret as i32, buffer)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_read(): worker thread interrupted while blocked on read()");
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

pub fn do_pwrite(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: PartialWriteRequest,
) -> Result<Message, WorkerThreadError> {
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
    match unsafe { handle_pwrite(&syscall_table, fd, buffer.as_ptr() as *const _, count, offset) } {
        ret if ret >= 0 => Ok(PartialWriteResponse::build(tid, ret as c_ssize_t)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_pwrite(): worker thread interrupted while blocked on pwrite()");
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

pub fn do_pread(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: PartialReadRequest,
) -> Result<Message, WorkerThreadError> {
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
    match unsafe { handle_pread(&syscall_table, fd, buffer.as_mut_ptr() as *mut _, count, offset) }
    {
        ret if ret >= 0 => Ok(PartialReadResponse::build(tid, ret as c_ssize_t, buffer)),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_pread(): worker thread interrupted while blocked on pread()");
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

pub fn do_linkat(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: LinkAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
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
    match unsafe {
        handle_linkat(&syscall_table, olddirfd, oldpath.as_ptr(), newdirfd, newpath.as_ptr(), flags)
    } {
        ret if ret == 0 => Ok(vec![LinkAtResponse::build(tid, ret)]),
        ret => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_linkat(): worker thread interrupted while blocked on linkat()");
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
pub fn do_fchdir(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: FileChdirRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("fchdir(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    debug!("libc::fchdir(): fd={fd:?}");
    match unsafe { handle_fchdir(&syscall_table, fd) } {
        0 => Ok(FileChdirResponse::build(tid)),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fchdir(): worker thread interrupted while blocked on fchdir()");
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

pub fn do_fchown(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
    request: FileChownRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("fchown(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let owner: u32 = request.owner;
    let group: u32 = request.group;

    debug!("libc::fchown(): fd={fd:?}, owner={owner:?}, group={group:?}");
    match unsafe { handle_fchown(&syscall_table, fd, owner, group) } {
        0 => Ok(FileChownResponse::build(tid)),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fchown(): worker thread interrupted while blocked on fchown()");
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

pub fn do_pipe(
    syscall_table: Arc<SystemCallRouteTable>,
    tid: ThreadIdentifier,
) -> Result<Message, WorkerThreadError> {
    trace!("pipe(): tid={tid:?}");

    let mut fds: [i32; 2] = [0; 2];

    debug!("libc::pipe(): fds={fds:?}");
    match unsafe { handle_pipe(&syscall_table, fds.as_mut_ptr()) } {
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
                error!("do_pipe(): worker thread interrupted while blocked on pipe()");
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

//==================================================================================================
// System Call Wrappers
//==================================================================================================

/// Handler for `libc::chdir()`.
unsafe fn handle_chdir(
    syscall_table: &SystemCallRouteTable,
    path: *const libc::c_char,
) -> libc::c_int {
    match &syscall_table.syscall_chdir {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(path) },
    }
}

/// Handler for `libc::close()`.
unsafe fn handle_close(syscall_table: &SystemCallRouteTable, fd: libc::c_int) -> libc::c_int {
    match &syscall_table.syscall_close {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd) },
    }
}

/// Handler for `libc::faccessat()`.
unsafe fn handle_faccessat(
    syscall_table: &SystemCallRouteTable,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    mode: libc::c_int,
    flags: libc::c_int,
) -> libc::c_int {
    match &syscall_table.syscall_faccessat {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(dirfd, pathname, mode, flags)
        },
    }
}

/// Handler for `libc::fdatasync()`.
unsafe fn handle_fdatasync(syscall_table: &SystemCallRouteTable, fd: libc::c_int) -> libc::c_int {
    match &syscall_table.syscall_fdatasync {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd) },
    }
}

/// Handler for `libc::getuid()`.
unsafe fn handle_getuid(syscall_table: &SystemCallRouteTable) -> libc::uid_t {
    match &syscall_table.syscall_getuid {
        SystemCallAction::Block => 0,
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn() },
    }
}

/// Handler for `libc::geteuid()`.
unsafe fn handle_geteuid(syscall_table: &SystemCallRouteTable) -> libc::uid_t {
    match &syscall_table.syscall_geteuid {
        SystemCallAction::Block => 0,
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn() },
    }
}

/// Handler for `libc::getgid()`.
unsafe fn handle_getgid(syscall_table: &SystemCallRouteTable) -> libc::gid_t {
    match &syscall_table.syscall_getgid {
        SystemCallAction::Block => 0,
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn() },
    }
}

/// Handler for `libc::getegid()`.
unsafe fn handle_getegid(syscall_table: &SystemCallRouteTable) -> libc::gid_t {
    match &syscall_table.syscall_getegid {
        SystemCallAction::Block => 0,
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn() },
    }
}

/// Handler for `libc::getcwd()`.
unsafe fn handle_getcwd(
    syscall_table: &SystemCallRouteTable,
    buf: *mut libc::c_char,
    size: libc::size_t,
) -> *mut libc::c_char {
    match &syscall_table.syscall_getcwd {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            ::core::ptr::null_mut()
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(buf, size) },
    }
}

/// Handler for `libc::fsync()`.
unsafe fn handle_fsync(syscall_table: &SystemCallRouteTable, fd: libc::c_int) -> libc::c_int {
    match &syscall_table.syscall_fsync {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd) },
    }
}

/// Handler for `libc::lseek()`.
unsafe fn handle_lseek(
    syscall_table: &SystemCallRouteTable,
    fd: libc::c_int,
    offset: libc::off_t,
    whence: libc::c_int,
) -> libc::off_t {
    match &syscall_table.syscall_lseek {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd, offset, whence) },
    }
}

/// Handler for `libc::ftruncate()`.
unsafe fn handle_ftruncate(
    syscall_table: &SystemCallRouteTable,
    fd: libc::c_int,
    length: libc::off_t,
) -> libc::c_int {
    match &syscall_table.syscall_ftruncate {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd, length) },
    }
}

/// Handler for `libc::write()`.
unsafe fn handle_write(
    syscall_table: &SystemCallRouteTable,
    fd: libc::c_int,
    buf: *const libc::c_void,
    count: libc::size_t,
) -> libc::ssize_t {
    match &syscall_table.syscall_write {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd, buf, count) },
    }
}

/// Handler for `libc::read()`.
unsafe fn handle_read(
    syscall_table: &SystemCallRouteTable,
    fd: libc::c_int,
    buf: *mut libc::c_void,
    count: libc::size_t,
) -> libc::ssize_t {
    match &syscall_table.syscall_read {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd, buf, count) },
    }
}

/// Handler for `libc::pwrite()`.
unsafe fn handle_pwrite(
    syscall_table: &SystemCallRouteTable,
    fd: libc::c_int,
    buf: *const libc::c_void,
    count: libc::size_t,
    offset: libc::off_t,
) -> libc::ssize_t {
    match &syscall_table.syscall_pwrite {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd, buf, count, offset) },
    }
}

/// Handler for `libc::pread()`.
unsafe fn handle_pread(
    syscall_table: &SystemCallRouteTable,
    fd: libc::c_int,
    buf: *mut libc::c_void,
    count: libc::size_t,
    offset: libc::off_t,
) -> libc::ssize_t {
    match &syscall_table.syscall_pread {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd, buf, count, offset) },
    }
}

/// Handler for `libc::linkat()`.
unsafe fn handle_linkat(
    syscall_table: &SystemCallRouteTable,
    olddirfd: libc::c_int,
    oldpath: *const libc::c_char,
    newdirfd: libc::c_int,
    newpath: *const libc::c_char,
    flags: libc::c_int,
) -> libc::c_int {
    match &syscall_table.syscall_linkat {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(olddirfd, oldpath, newdirfd, newpath, flags)
        },
    }
}

/// Handler for `libc::fchdir()`.
unsafe fn handle_fchdir(syscall_table: &SystemCallRouteTable, fd: libc::c_int) -> libc::c_int {
    match &syscall_table.syscall_fchdir {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd) },
    }
}

/// Handler for `libc::fchown()`.
unsafe fn handle_fchown(
    syscall_table: &SystemCallRouteTable,
    fd: libc::c_int,
    owner: libc::uid_t,
    group: libc::gid_t,
) -> libc::c_int {
    match &syscall_table.syscall_fchown {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(fd, owner, group) },
    }
}

/// Handler for `libc::pipe()`.
unsafe fn handle_pipe(
    syscall_table: &SystemCallRouteTable,
    pipefd: *mut libc::c_int,
) -> libc::c_int {
    match &syscall_table.syscall_pipe {
        SystemCallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SystemCallAction::Forward(syscall_fn) => unsafe { syscall_fn(pipefd) },
    }
}
