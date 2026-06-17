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
    time::LibcTimeSpec,
};
use ::alloc::ffi::CString;
use ::core::ffi;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::{
    fcntl::{
        atflags::{
            AT_EACCESS,
            AT_FDCWD,
            AT_REMOVEDIR,
            AT_SYMLINK_NOFOLLOW,
        },
        file_access_mode::{
            O_EXEC,
            O_RDONLY,
            O_RDWR,
            O_SEARCH,
            O_WRONLY,
        },
        file_advice::{
            POSIX_FADV_DONTNEED,
            POSIX_FADV_NOREUSE,
            POSIX_FADV_NORMAL,
            POSIX_FADV_RANDOM,
            POSIX_FADV_SEQUENTIAL,
            POSIX_FADV_WILLNEED,
        },
        file_control_request::{
            F_DUPFD,
            F_DUPFD_CLOEXEC,
            F_DUPFD_CLOFORK,
            F_GETFD,
            F_GETFL,
            F_GETLK,
            F_GETOWN,
            F_SETFD,
            F_SETFL,
            F_SETLK,
            F_SETLKW,
            F_SETOWN,
        },
        file_creation_flags::{
            O_CLOEXEC,
            O_CLOFORK,
            O_CREAT,
            O_DIRECTORY,
            O_EXCL,
            O_NOCTTY,
            O_NOFOLLOW,
            O_TRUNC,
        },
        file_status_flags::{
            O_APPEND,
            O_NONBLOCK,
            O_SYNC,
        },
    },
    ffi::c_int,
    limits::PATH_MAX,
    sys_stat::{
        file_mode::{
            S_IRGRP,
            S_IROTH,
            S_IRUSR,
            S_IRWXG,
            S_IRWXO,
            S_IRWXU,
            S_IWGRP,
            S_IWOTH,
            S_IWUSR,
            S_IXGRP,
            S_IXOTH,
            S_IXUSR,
        },
        stat,
    },
    sys_types::{
        mode_t,
        off_t,
    },
    time::timespec,
};
use ::syscall::{
    fcntl::message::{
        FileAdvisoryInformationRequest,
        FileAdvisoryInformationResponse,
        FileControlRequest,
        FileControlResponse,
        FileSpaceControlRequest,
        FileSpaceControlResponse,
        OpenAtRequest,
        OpenAtResponse,
        RenameAtRequest,
        RenameAtResponse,
        UnlinkAtRequest,
        UnlinkAtResponse,
    },
    message::MessagePartitioner,
    sys::stat::message::{
        FileChmodAtRequest,
        FileChmodAtResponse,
        FileChmodRequest,
        FileChmodResponse,
        FileStatAtRequest,
        FileStatAtResponse,
        FileStatRequest,
        MakeDirectoryAtRequest,
        MakeDirectoryAtResponse,
        UpdateFileAccessTimeAtRequest,
        UpdateFileAccessTimeAtResponse,
        UpdateFileAccessTimeRequest,
        UpdateFileAccessTimeResponse,
    },
    unistd::message::{
        FileChownAtRequest,
        FileChownAtResponse,
        ReadLinkAtRequest,
        ReadLinkAtResponse,
        SymbolicLinkAtRequest,
        SymbolicLinkAtResponse,
    },
};
use sysapi::fcntl::file_descriptor_flags::{
    FD_CLOEXEC,
    FD_CLOFORK,
};

//==================================================================================================
// do_openat
//==================================================================================================

pub fn do_openat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: OpenAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("openat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let flags: ffi::c_int = request.flags;
    let mode: mode_t = request.mode;

    let pathname: CString = match CString::new(request.pathname.as_str()) {
        Ok(pathname) => pathname,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);
    let flags: LibcFileOpenFlags = match LibcFileOpenFlags::try_from_nanvix_flags(flags) {
        Ok(flags) => flags,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };
    let mode: LibcFileMode = match LibcFileMode::try_from(mode) {
        Ok(mode) => mode,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    debug!(
        "libc::openat(): dirfd={:?}, pathname={pathname:?}, flags={:?}, mode={:?}",
        dirfd.inner(),
        flags.inner(),
        mode.inner()
    );
    match unsafe {
        handle_openat(syscall_table, dirfd.inner(), pathname.as_ptr(), flags.inner(), mode.inner())
    } {
        fd if fd >= 0 => {
            debug!("libc::openat(): fd={fd:?}");
            Ok(vec![OpenAtResponse::build(
                tid,
                fd,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_openat(): worker thread interrupted while blocked on openat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::openat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_openat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_unlink_at
//==================================================================================================

pub fn do_unlinkat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: UnlinkAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("unlinkat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let flags: c_int = request.flags;

    let pathname: CString = match CString::new(request.pathname.as_str()) {
        Ok(pathname) => pathname,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);
    let flags: libc::c_int = if flags == AT_REMOVEDIR {
        libc::AT_REMOVEDIR
    } else {
        0
    };

    debug!("libc::unlinkat(): dirfd={:?}, pathname={pathname:?}, flags={flags:?}", dirfd.inner(),);
    match unsafe {
        handle_unlinkat(
            syscall_table,
            dirfd.inner(),
            pathname.as_bytes().as_ptr() as *const i8,
            flags,
        )
    } {
        ret if ret == 0 => {
            debug!("libc::unlinkat(): success");
            Ok(vec![UnlinkAtResponse::build(
                tid,
                ret,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        errno => {
            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_unlinkat(): worker thread interrupted while blocked on unlinkat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::unlinkat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_unlinkat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_rename_at
//==================================================================================================

pub fn do_renameat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: RenameAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("renameat(): tid={tid:?}, request={request:?}");

    let olddirfd: i32 = request.olddirfd;
    let newdirfd: i32 = request.newdirfd;

    let oldpath: CString = match CString::new(request.oldpath.as_str()) {
        Ok(oldpath) => oldpath,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let newpath: CString = match CString::new(request.newpath.as_str()) {
        Ok(newpath) => newpath,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let olddirfd: LibcAtFlags = LibcAtFlags::from_dirfd(olddirfd);
    let newdirfd: LibcAtFlags = LibcAtFlags::from_dirfd(newdirfd);

    debug!(
        "libc::renameat(): olddirfd={:?}, oldpath={oldpath:?}, newdirfd={:?}, newpath={newpath:?}",
        olddirfd.inner(),
        newdirfd.inner(),
    );
    match unsafe {
        handle_renameat(
            syscall_table,
            olddirfd.inner(),
            oldpath.as_bytes().as_ptr() as *const i8,
            newdirfd.inner(),
            newpath.as_bytes().as_ptr() as *const i8,
        )
    } {
        ret if ret == 0 => {
            debug!("libc::renameat(): success");
            Ok(vec![RenameAtResponse::build(
                tid,
                ret,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        errno => {
            debug!("libc::renameat(): errno={errno:?}");

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_renameat(): worker thread interrupted while blocked on renameat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::renameat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_renameat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_fstatat
//==================================================================================================

#[cfg_attr(target_arch = "x86_64", allow(clippy::useless_conversion))]
pub fn do_fstat_at<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileStatAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("fstatat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);
    let flag: libc::c_int = if request.flag & AT_SYMLINK_NOFOLLOW != 0 {
        libc::AT_SYMLINK_NOFOLLOW
    } else {
        0
    };
    let path: CString = match CString::new(request.path.as_str()) {
        Ok(c_string) => c_string,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let mut st: libc::stat = unsafe { core::mem::zeroed() };

    debug!("libc::fstatat(): dirfd={:?}, path={path:?}, flag={:?}", dirfd.inner(), flag);
    match unsafe {
        handle_fstatat(
            syscall_table,
            dirfd.inner(),
            path.as_ptr(),
            &mut st as *mut libc::stat,
            flag,
        )
    } {
        0 => {
            debug!("libc::fstatat(): success");

            let stat = stat {
                st_dev: st.st_dev,
                st_ino: st.st_ino,
                st_mode: st.st_mode,
                st_nlink: st.st_nlink,
                st_uid: st.st_uid,
                st_gid: st.st_gid,
                st_rdev: st.st_rdev,
                st_size: st.st_size,
                st_atim: timespec {
                    tv_sec: st.st_atime,
                    tv_nsec: match st.st_atime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return Ok(vec![crate::build_error(tid, ErrorCode::ValueOutOfRange)]);
                        },
                    },
                },
                st_mtim: timespec {
                    tv_sec: st.st_mtime,
                    tv_nsec: match st.st_mtime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return Ok(vec![crate::build_error(tid, ErrorCode::ValueOutOfRange)]);
                        },
                    },
                },
                st_ctim: timespec {
                    tv_sec: st.st_ctime,
                    tv_nsec: match st.st_ctime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return Ok(vec![crate::build_error(tid, ErrorCode::ValueOutOfRange)]);
                        },
                    },
                },
                st_blksize: st.st_blksize,
                st_blocks: st.st_blocks,
            };

            // Print size of stat structure.
            debug!("libc::fstatat(): size of stat={:?}", core::mem::size_of::<stat>());
            let response = FileStatAtResponse::new(stat);

            match response.into_parts(tid, ::syscall::LINUXD, MessageType::Ikc) {
                Ok(messages) => Ok(messages),
                Err(e) => Ok(vec![crate::build_error(tid, e.code)]),
            }
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fstat_at(): worker thread interrupted while blocked on fstatat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::fstatat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_fstat_at(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_posix_fallocate
//==================================================================================================

pub fn do_posix_fallocate<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileSpaceControlRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("posix_fallocate(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let offset: off_t = request.offset;
    let len: off_t = request.len;

    debug!("libc::posix_fallocate(): fd={fd:?}, offset={offset:?}, len={len:?}");
    match unsafe { handle_posix_fallocate(syscall_table, fd, offset, len) } {
        0 => {
            debug!("libc::posix_fallocate(): success");
            Ok(FileSpaceControlResponse::build(tid, 0, ::syscall::LINUXD, MessageType::Ikc))
        },
        errno => {
            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!(
                    "do_posix_fallocate(): worker thread interrupted while blocked on \
                     posix_fallocate()"
                );
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::posix_fallocate(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_posix_fallocate(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(crate::build_error(tid, error))
        },
    }
}

//==================================================================================================
// do_posix_fadvise
//==================================================================================================

pub fn do_posix_fadvise<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileAdvisoryInformationRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("posix_fadvise(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let offset: off_t = request.offset;
    let len: off_t = request.len;
    let advice: LibcFileAdvice = match LibcFileAdvice::try_from(request.advice) {
        Ok(advice) => advice,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };

    debug!(
        "libc::posix_fadvise(): fd={fd:?}, offset={offset:?}, len={len:?}, advice={:?}",
        advice.inner()
    );
    match unsafe { handle_posix_fadvise(syscall_table, fd, offset, len, advice.inner()) } {
        0 => {
            debug!("libc::posix_fadvise(): success");
            Ok(FileAdvisoryInformationResponse::build(tid, 0, ::syscall::LINUXD, MessageType::Ikc))
        },
        errno => {
            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!(
                    "do_posix_fadvise(): worker thread interrupted while blocked on \
                     posix_fadvise()"
                );
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::posix_fadvise(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_posix_fadvise(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(crate::build_error(tid, error))
        },
    }
}

//==================================================================================================
// do_fstat()
//==================================================================================================

#[cfg_attr(target_arch = "x86_64", allow(clippy::useless_conversion))]
pub fn do_fstat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileStatRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("fstatat(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    let mut st: libc::stat = unsafe { core::mem::zeroed() };

    debug!("libc::fstat(): fd={fd:?}");
    match unsafe { handle_fstat(syscall_table, fd, &mut st) } {
        0 => {
            debug!("libc::fstatat(): success");

            let stat = stat {
                st_dev: st.st_dev,
                st_ino: st.st_ino,
                st_mode: st.st_mode,
                st_nlink: st.st_nlink,
                st_uid: st.st_uid,
                st_gid: st.st_gid,
                st_rdev: st.st_rdev,
                st_size: st.st_size,
                st_atim: timespec {
                    tv_sec: st.st_atime,
                    tv_nsec: match st.st_atime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return Ok(vec![crate::build_error(tid, ErrorCode::ValueOutOfRange)]);
                        },
                    },
                },
                st_mtim: timespec {
                    tv_sec: st.st_mtime,
                    tv_nsec: match st.st_mtime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return Ok(vec![crate::build_error(tid, ErrorCode::ValueOutOfRange)]);
                        },
                    },
                },
                st_ctim: timespec {
                    tv_sec: st.st_ctime,
                    tv_nsec: match st.st_ctime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return Ok(vec![crate::build_error(tid, ErrorCode::ValueOutOfRange)]);
                        },
                    },
                },
                st_blksize: st.st_blksize,
                st_blocks: st.st_blocks,
            };

            // Print size of stat structure.
            debug!("libc::fstatat(): size of stat={:?}", core::mem::size_of::<stat>());
            let response = FileStatAtResponse::new(stat);

            match response.into_parts(tid, ::syscall::LINUXD, MessageType::Ikc) {
                Ok(messages) => Ok(messages),
                Err(e) => Ok(vec![crate::build_error(tid, e.code)]),
            }
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fstat(): worker thread interrupted while blocked on fstat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::fstatat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_fstat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_symlinkat()
//==================================================================================================

pub fn do_symlinkat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: SymbolicLinkAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("symlinkat(): tid={tid:?}, request={request:?}");

    let target: CString = match CString::new(request.target.as_str()) {
        Ok(target) => target,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let newdirfd: i32 = request.dirfd;
    let newdirfd: LibcAtFlags = LibcAtFlags::from_dirfd(newdirfd);

    let linkpath: CString = match CString::new(request.linkpath.as_str()) {
        Ok(linkpath) => linkpath,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    debug!(
        "libc::symlinkat(): oldpath={target:?}, newdirfd={:?}, newpath={linkpath:?}",
        newdirfd.inner(),
    );
    match unsafe {
        handle_symlinkat(syscall_table, target.as_ptr(), newdirfd.inner(), linkpath.as_ptr())
    } {
        0 => {
            debug!("libc::symlinkat(): success");
            Ok(vec![SymbolicLinkAtResponse::build(
                tid,
                0,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_symlinkat(): worker thread interrupted while blocked on symlinkat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::symlinkat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_symlinkat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_readlinkat()
//==================================================================================================

pub fn do_readlinkat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: ReadLinkAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("readlinkat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    // TODO: Have a system-wide constant for this.
    let mut buf: Vec<u8> = vec![0u8; PATH_MAX];

    debug!(
        "libc::readlinkat(): dirfd={:?}, path={path:?}, capacity={:?}",
        dirfd.inner(),
        buf.capacity()
    );
    match unsafe {
        handle_readlinkat(
            syscall_table,
            dirfd.inner(),
            path.as_ptr(),
            buf.as_mut_ptr() as *mut i8,
            buf.capacity(),
        )
    } {
        len if len >= 0 => {
            debug!("libc::readlinkat(): (len={len:?})");

            buf.truncate(len as usize);

            let response: ReadLinkAtResponse = match ReadLinkAtResponse::new(buf) {
                Ok(response) => response,
                Err(e) => return Ok(vec![crate::build_error(tid, e.code)]),
            };

            match response.into_parts(tid, ::syscall::LINUXD, MessageType::Ikc) {
                Ok(messages) => Ok(messages),
                Err(e) => Ok(vec![crate::build_error(tid, e.code)]),
            }
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_readlinkat(): worker thread interrupted while blocked on readlinkat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::readlinkat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_readlinkat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_mkdirat()
//==================================================================================================

pub fn do_mkdirat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: MakeDirectoryAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("mkdirat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);

    let pathname: CString = match CString::new(request.pathname.as_str()) {
        Ok(pathname) => pathname,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let mode: LibcFileMode = match LibcFileMode::try_from(request.mode) {
        Ok(mode) => mode,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    debug!(
        "libc::mkdirat(): dirfd={:?}, pathname={pathname:?}, mode={:?}",
        dirfd.inner(),
        mode.inner()
    );
    match unsafe { handle_mkdirat(syscall_table, dirfd.inner(), pathname.as_ptr(), mode.inner()) } {
        0 => {
            debug!("libc::mkdirat(): success");
            Ok(vec![MakeDirectoryAtResponse::build(
                tid,
                0,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_mkdirat(): worker thread interrupted while blocked on mkdirat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::mkdirat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_mkdirat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_utimensat()
//==================================================================================================

pub fn do_utimensat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: UpdateFileAccessTimeAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("utimensat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let times: [timespec; 2] = request.times;

    let libc_times: [libc::timespec; 2] = [
        Into::<LibcTimeSpec>::into(times[0]).into(),
        Into::<LibcTimeSpec>::into(times[1]).into(),
    ];

    let flag: libc::c_int = if request.flag & AT_SYMLINK_NOFOLLOW != 0 {
        libc::AT_SYMLINK_FOLLOW
    } else {
        0
    };

    debug!(
        "libc::utimensat(): dirfd={:?}, path={path:?}, flag={:?}, times[0].tv_sec={:?}, \
         times[0].tv_nsec={:?}, times[1].tv_sec={:?}, times[1].tv_nsec={:?}",
        dirfd.inner(),
        flag,
        libc_times[0].tv_sec,
        libc_times[0].tv_nsec,
        libc_times[1].tv_sec,
        libc_times[1].tv_nsec
    );
    match unsafe {
        handle_utimensat(syscall_table, dirfd.inner(), path.as_ptr(), libc_times.as_ptr(), flag)
    } {
        0 => {
            debug!("libc::utimensat(): success");
            Ok(vec![UpdateFileAccessTimeAtResponse::build(
                tid,
                0,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_utimensat(): worker thread interrupted while blocked on utimensat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::utimensat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_utimensat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_futimens()
//==================================================================================================

pub fn do_futimens<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: UpdateFileAccessTimeRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("futimens(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;

    let times: [timespec; 2] = request.times;

    let libc_times: [libc::timespec; 2] = [
        Into::<LibcTimeSpec>::into(times[0]).into(),
        Into::<LibcTimeSpec>::into(times[1]).into(),
    ];

    debug!(
        "libc::futimens(): fd={fd:?}, times[0].tv_sec={:?}, times[0].tv_nsec={:?}, \
         times[1].tv_sec={:?}, times[1].tv_nsec={:?}",
        libc_times[0].tv_sec, libc_times[0].tv_nsec, libc_times[1].tv_sec, libc_times[1].tv_nsec
    );
    match unsafe { handle_futimens(syscall_table, fd, libc_times.as_ptr()) } {
        0 => {
            debug!("libc::futimens(): success");
            Ok(UpdateFileAccessTimeResponse::build(tid, 0, ::syscall::LINUXD, MessageType::Ikc))
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_futimens(): worker thread interrupted while blocked on futimens()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::futimens(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_futimens(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(crate::build_error(tid, error))
        },
    }
}

//==================================================================================================
// do_fcntl()
//==================================================================================================

pub fn do_fcntl<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileControlRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("fcntl(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let cmd: LibcFileControlCommand = match LibcFileControlCommand::try_from(request.cmd) {
        Ok(cmd) => cmd,
        Err(e) => return Ok(crate::build_error(tid, e.code)),
    };

    debug!("libc::fcntl(): fd={fd:?}, cmd={:?}, arg={:?}", cmd.inner(), { request.arg });

    let libc_arg: c_int = match cmd.inner() {
        libc::F_DUPFD => request.arg,
        libc::F_DUPFD_CLOEXEC => request.arg,
        libc::F_GETFD => request.arg,
        libc::F_SETFD => match LibcFileDescriptorFlags::try_from_nanvix_flags(request.arg) {
            Ok(flags) => flags.inner(),
            Err(error) => {
                error!("do_fcntl(): {error:?} (cmd={cmd:#x?}, arg={:?})", { request.arg });
                return Ok(crate::build_error(tid, error.code));
            },
        },
        libc::F_GETFL => 0,
        libc::F_SETFL => {
            match LibcFileStatusFlags::try_from_nanvix_flags(
                request.arg & LibcFileStatusFlags::nanvix_mask(),
            ) {
                Ok(flags) => flags.inner(),
                Err(error) => {
                    error!("do_fcntl(): {error:?} (cmd={cmd:#x?}), arg={:?})", { request.arg });
                    return Ok(crate::build_error(tid, error.code));
                },
            }
        },
        libc::F_GETOWN => 0,
        libc::F_SETOWN => {
            let arg: i32 = request.arg;
            if arg < 0 {
                if arg == -1 {
                    error!("do_fcntl(): invalid owner (cmd={cmd:#x?}, arg={arg:?})");
                    return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
                } else {
                    -arg
                }
            } else {
                arg
            }
        },
        libc::F_GETLK => {
            let reason: &str = "unsupported file lock command";
            error!("do_fcntl(): {reason:?} (cmd={cmd:#x?}, arg={:?})", { request.arg });
            return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
        },
        libc::F_SETLK => {
            let reason: &str = "unsupported file lock command";
            error!("do_fcntl(): {reason:?} (cmd={cmd:#x?}, arg={:?})", { request.arg });
            return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
        },
        libc::F_SETLKW => {
            let reason: &str = "unsupported file lock command";
            error!("do_fcntl(): {reason:?} (cmd={cmd:#x?}, arg={:?})", { request.arg });
            return Ok(crate::build_error(tid, ErrorCode::InvalidArgument));
        },
        unsupported_cmd => {
            // The following statement is unreachable because any unsupported commands were already
            // discarded when converting `request.cmd` to `LibcFileControlCommand`.
            unreachable!(
                "do_fcntl(): unsupported file control command \
                 (unsupported_cmd={unsupported_cmd:#x?}, cmd={cmd:#x?}, arg={:?})",
                { request.arg }
            );
        },
    };

    let ret: i32 = unsafe { handle_fcntl(syscall_table, fd, cmd.inner(), libc_arg) };

    match cmd.inner() {
        libc::F_DUPFD | libc::F_DUPFD_CLOEXEC => {
            if ret >= 0 {
                debug!("libc::fcntl(): F_DUPFD | F_DUPFD_CLOEXEC success");
                Ok(FileControlResponse::build(tid, ret, ::syscall::LINUXD, MessageType::Ikc))
            } else {
                let errno: i32 = unsafe { *libc::__errno_location() };

                // Check if the thread has been interrupted.
                if errno == libc::EINTR {
                    error!("do_fcntl(): worker thread interrupted while blocked on fcntl()");
                    return Err(WorkerThreadError::Interrupted);
                }

                error!("libc::fcntl(): errno={errno:?} (cmd={cmd:#x?}, arg={libc_arg})");
                let error: ErrorCode = match ErrorCode::try_from(errno) {
                    Ok(error) => error,
                    Err(_) => {
                        let reason: &str = "unknown error code";
                        warn!("do_fcntl(): {reason} (errno={errno:?})");
                        ErrorCode::ValueOutOfRange
                    },
                };
                Ok(crate::build_error(tid, error))
            }
        },
        libc::F_GETFD => {
            if ret != -1 {
                debug!("libc::fcntl(): F_GETFD success");
                let nanvix_file_descritor_flags: c_int = LibcFileDescriptorFlags(ret)
                    .try_into_nanvix_flags()
                    .unwrap_or_else(|_| panic!("unexpected file descriptor flags: {ret:?}"));
                Ok(FileControlResponse::build(
                    tid,
                    nanvix_file_descritor_flags,
                    ::syscall::LINUXD,
                    MessageType::Ikc,
                ))
            } else {
                let errno: i32 = unsafe { *libc::__errno_location() };

                // Check if the thread has been interrupted.
                if errno == libc::EINTR {
                    error!("do_fcntl(): worker thread interrupted while blocked on fcntl()");
                    return Err(WorkerThreadError::Interrupted);
                }

                error!("libc::fcntl(): errno={errno:?} (cmd={cmd:#x?}, arg={libc_arg})");
                let error: ErrorCode = match ErrorCode::try_from(errno) {
                    Ok(error) => error,
                    Err(_) => {
                        let reason: &str = "unknown error code";
                        warn!("do_fcntl(): {reason} (errno={errno:?})");
                        ErrorCode::ValueOutOfRange
                    },
                };
                Ok(crate::build_error(tid, error))
            }
        },
        libc::F_GETFL => {
            if ret != -1 {
                debug!("libc::fcntl(): F_GETFL success");
                let libc_file_status_flags: LibcFileStatusFlags =
                    LibcFileStatusFlags(ret & LibcFileStatusFlags::libc_mask());

                let nanvix_file_status_flags: c_int = libc_file_status_flags
                    .try_into_nanvix_flags()
                    .unwrap_or_else(|_| {
                        panic!("unexpected file status flags: {libc_file_status_flags:?}")
                    });

                let libc_file_creation_flags: LibcFileCreationFlags =
                    LibcFileCreationFlags(ret & LibcFileCreationFlags::libc_mask());

                let nanvix_file_creation_flags: c_int = libc_file_creation_flags
                    .try_into_nanvix_flags()
                    .unwrap_or_else(|_| {
                        panic!("unexpected file creation flags: {libc_file_creation_flags:?}")
                    });

                debug_assert_eq!(
                    nanvix_file_status_flags & nanvix_file_creation_flags,
                    0,
                    "file status flags and file creation flags should not overlap"
                );

                Ok(FileControlResponse::build(
                    tid,
                    nanvix_file_status_flags | nanvix_file_creation_flags,
                    ::syscall::LINUXD,
                    MessageType::Ikc,
                ))
            } else {
                let errno: i32 = unsafe { *libc::__errno_location() };

                // Check if the thread has been interrupted.
                if errno == libc::EINTR {
                    error!("do_fcntl(): worker thread interrupted while blocked on fcntl()");
                    return Err(WorkerThreadError::Interrupted);
                }

                error!("libc::fcntl(): errno={errno:?} (cmd={cmd:#x?}, arg={libc_arg})");
                let error: ErrorCode = match ErrorCode::try_from(errno) {
                    Ok(error) => error,
                    Err(_) => {
                        let reason: &str = "unknown error code";
                        warn!("do_fcntl(): {reason} (errno={errno:?})");
                        ErrorCode::ValueOutOfRange
                    },
                };
                Ok(crate::build_error(tid, error))
            }
        },
        libc::F_GETOWN => {
            if ret != -1 {
                debug!("libc::fcntl(): F_GETOWN success");
                Ok(FileControlResponse::build(tid, ret, ::syscall::LINUXD, MessageType::Ikc))
            } else {
                // The following statement is unreachable because `libc::fcntl()` should never
                // return -1 for `F_GETOWN`.
                unreachable!(
                    "do_fcntl(): unexpected return for F_GETOWN (cmd={cmd:#x?}, arg={libc_arg}, \
                     ret={ret:?}"
                )
            }
        },
        libc::F_SETFD | libc::F_SETFL | libc::F_SETOWN => {
            if ret == 0 {
                debug!("libc::fcntl(): libc::F_SETFD | libc::F_SETFL | libc::F_SETOWN success");
                Ok(FileControlResponse::build(tid, ret, ::syscall::LINUXD, MessageType::Ikc))
            } else if ret == -1 {
                let errno: i32 = unsafe { *libc::__errno_location() };

                // Check if the thread has been interrupted.
                if errno == libc::EINTR {
                    error!("do_fcntl(): worker thread interrupted while blocked on fcntl()");
                    return Err(WorkerThreadError::Interrupted);
                }

                error!("libc::fcntl(): errno={errno:?} (cmd={cmd:#x?}, arg={libc_arg})");
                let error: ErrorCode = match ErrorCode::try_from(errno) {
                    Ok(error) => error,
                    Err(_) => {
                        let reason: &str = "unknown error code";
                        warn!("do_fcntl(): {reason} (errno={errno:?})");
                        ErrorCode::ValueOutOfRange
                    },
                };
                Ok(crate::build_error(tid, error))
            } else {
                // The following statement is unreachable because `libc::fcntl()` should return
                // either 0 on success or -1 on error for `F_SETFD`, `F_SETFL`, and `F_SETOWN`.
                unreachable!(
                    "do_fcntl(): unexpected return for F_SETFD | F_SETFL | F_SETOWN \
                     (cmd={cmd:#x?}, arg={libc_arg}, ret={ret:?}"
                )
            }
        },
        unsupported_cmd => {
            // The following statement is unreachable because any unsupported were not passed in
            // to the underlying libc.
            unreachable!(
                "do_fcntl(): unsupported file control command \
                 (unsupported_cmd={unsupported_cmd:#x?}, cmd={cmd:#x?}, arg={libc_arg})"
            )
        },
    }
}

//==================================================================================================
// do_fchownat()
//==================================================================================================

pub fn do_fchownat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileChownAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("fchownat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let owner: u32 = request.owner;
    let group: u32 = request.group;
    let flag: LibcAtFlags = LibcAtFlags::from(request.flag);

    debug!(
        "libc::fchownat(): dirfd={:?}, path={path:?}, owner={owner:?}, group={group:?}, flag={:?}",
        dirfd.inner(),
        flag.inner()
    );
    match unsafe {
        handle_fchownat(syscall_table, dirfd.inner(), path.as_ptr(), owner, group, flag.inner())
    } {
        0 => {
            debug!("libc::fchownat(): success");
            Ok(vec![FileChownAtResponse::build(
                tid,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        _ => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fchownat(): worker thread interrupted while blocked on fchownat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::fchownat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_fchownat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// do_fchmod
//==================================================================================================

pub fn do_fchmod<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileChmodRequest,
) -> Result<Message, WorkerThreadError> {
    trace!("fchmod(): tid={tid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let mode: u32 = request.mode;

    debug!("libc::fchmod(): fd={fd:?}, mode={mode:?}");
    match unsafe { handle_fchmod(syscall_table, fd, mode) } {
        0 => Ok(FileChmodResponse::build(tid, ::syscall::LINUXD, MessageType::Ikc)),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fchmod(): worker thread interrupted while blocked on fchmod()");
                return Err(WorkerThreadError::Interrupted);
            }

            Ok(crate::build_error(
                tid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            ))
        },
        ret => unreachable!("libc::fchmod() returned an invalid value ({ret:?})"),
    }
}

//==================================================================================================
// do_fchmodat()
//==================================================================================================

pub fn do_fchmodat<T>(
    syscall_table: &SyscallTable<T>,
    tid: ThreadIdentifier,
    request: FileChmodAtRequest,
) -> Result<Vec<Message>, WorkerThreadError> {
    trace!("fchmodat(): tid={tid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from_dirfd(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let mode: LibcFileMode = match LibcFileMode::try_from(request.mode) {
        Ok(mode) => mode,
        Err(_) => return Ok(vec![crate::build_error(tid, ErrorCode::InvalidMessage)]),
    };

    let flag: LibcAtFlags = LibcAtFlags::from(request.flag);

    debug!(
        "libc::fchmodat(): dirfd={:?}, path={:?}, mode={:?}, flags={:?}",
        dirfd.inner(),
        path,
        mode.inner(),
        flag.inner()
    );
    match unsafe {
        handle_fchmodat(syscall_table, dirfd.inner(), path.as_ptr(), mode.inner(), flag.inner())
    } {
        0 => {
            debug!("libc::fchmodat(): success");
            Ok(vec![FileChmodAtResponse::build(
                tid,
                ::syscall::LINUXD,
                MessageType::Ikc,
            )])
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };

            // Check if the thread has been interrupted.
            if errno == libc::EINTR {
                error!("do_fchmodat(): worker thread interrupted while blocked on fchmodat()");
                return Err(WorkerThreadError::Interrupted);
            }

            error!("libc::fchmodat(): errno={errno:?}");
            let error: ErrorCode = match ErrorCode::try_from(errno) {
                Ok(error) => error,
                Err(_) => {
                    let reason: &str = "unknown error code";
                    warn!("do_fchmodat(): {reason} (errno={errno:?})");
                    ErrorCode::ValueOutOfRange
                },
            };
            Ok(vec![crate::build_error(tid, error)])
        },
    }
}

//==================================================================================================
// LibcFileOpenFlags
//==================================================================================================

struct LibcFileOpenFlags(libc::c_int);

impl LibcFileOpenFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn nanvix_mask() -> c_int {
        LibcFileAccessModeFlags::nanvix_mask()
            | LibcFileCreationFlags::nanvix_mask()
            | LibcFileStatusFlags::nanvix_mask()
    }

    fn try_from_nanvix_flags(flags: c_int) -> Result<Self, Error> {
        // Check if any unsupported flags are set.
        if flags & !Self::nanvix_mask() != 0 {
            let reason: &str = "unsupported file open flags";
            error!("do_fcntl(): {reason} (flags={flags:#x?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let access_mode_flags: LibcFileAccessModeFlags =
            LibcFileAccessModeFlags::try_from_nanvix_flags(
                flags & LibcFileAccessModeFlags::nanvix_mask(),
            )?;
        let creation_flags: LibcFileCreationFlags = LibcFileCreationFlags::try_from_nanvix_flags(
            flags & LibcFileCreationFlags::nanvix_mask(),
        )?;
        let status_flags: LibcFileStatusFlags =
            LibcFileStatusFlags::try_from_nanvix_flags(flags & LibcFileStatusFlags::nanvix_mask())?;

        Ok(LibcFileOpenFlags(
            access_mode_flags.inner() | creation_flags.inner() | status_flags.inner(),
        ))
    }
}

//==================================================================================================
// LibcFileAccessModeFlags
//==================================================================================================

#[derive(Debug)]
struct LibcFileAccessModeFlags(libc::c_int);

impl LibcFileAccessModeFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn nanvix_mask() -> c_int {
        O_RDONLY | O_WRONLY | O_RDWR | O_EXEC | O_SEARCH
    }

    fn try_from_nanvix_flags(flags: c_int) -> Result<Self, Error> {
        match flags {
            O_RDONLY => Ok(LibcFileAccessModeFlags(libc::O_RDONLY)),
            O_WRONLY => Ok(LibcFileAccessModeFlags(libc::O_WRONLY)),
            O_RDWR => Ok(LibcFileAccessModeFlags(libc::O_RDWR)),
            O_EXEC => {
                let reason: &str = "O_EXEC|O_SEARCH are not supported by libc";
                error!("do_fcntl(): {reason} (flags={flags:#x?})");
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
            _ => {
                let reason: &str = "unsupported file access mode flags";
                error!("do_fcntl(): {reason} (flags={flags:#x?})");
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
        }
    }
}

//==================================================================================================
// LibcFileCreationFlags
//==================================================================================================

#[derive(Debug)]
struct LibcFileCreationFlags(libc::c_int);

impl LibcFileCreationFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn nanvix_mask() -> c_int {
        O_CREAT | O_TRUNC | O_EXCL | O_NOCTTY | O_NOFOLLOW | O_DIRECTORY | O_CLOEXEC | O_CLOFORK
    }

    fn libc_mask() -> libc::c_int {
        libc::O_CREAT
            | libc::O_TRUNC
            | libc::O_EXCL
            | libc::O_NOCTTY
            | libc::O_NOFOLLOW
            | libc::O_DIRECTORY
            | libc::O_CLOEXEC
    }

    fn try_from_nanvix_flags(flags: c_int) -> Result<Self, Error> {
        // Check if any unsupported flags are set.
        if flags & !Self::nanvix_mask() != 0 {
            let reason: &str = "unsupported file creation flags";
            error!("do_fcntl(): {reason} (flags={flags:#x?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let mut libc_flags: libc::c_int = 0;

        if flags & O_CREAT != 0 {
            libc_flags |= libc::O_CREAT;
        }
        if flags & O_TRUNC != 0 {
            libc_flags |= libc::O_TRUNC;
        }
        if flags & O_EXCL != 0 {
            libc_flags |= libc::O_EXCL;
        }
        if flags & O_NOCTTY != 0 {
            libc_flags |= libc::O_NOCTTY;
        }
        if flags & O_NOFOLLOW != 0 {
            libc_flags |= libc::O_NOFOLLOW;
        }
        if flags & O_DIRECTORY != 0 {
            libc_flags |= libc::O_DIRECTORY;
        }
        if flags & O_CLOEXEC != 0 {
            libc_flags |= libc::O_CLOEXEC;
        }
        if flags & O_CLOFORK != 0 {
            let reason: &str = "O_CLOFORK is not supported by libc";
            error!("do_fcntl(): {reason} (flags={flags:#x?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(LibcFileCreationFlags(libc_flags))
    }

    fn try_into_nanvix_flags(&self) -> Result<c_int, Error> {
        // Check if any unsupported flags are set.
        if self.0 & !Self::libc_mask() != 0 {
            let reason: &str = "unsupported file creation flags";
            error!("do_fcntl(): {reason} (flags={:#x?})", self.0);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let mut flags: c_int = 0;

        if self.0 & libc::O_CREAT != 0 {
            flags |= O_CREAT;
        }
        if self.0 & libc::O_TRUNC != 0 {
            flags |= O_TRUNC;
        }
        if self.0 & libc::O_EXCL != 0 {
            flags |= O_EXCL;
        }
        if self.0 & libc::O_NOCTTY != 0 {
            flags |= O_NOCTTY;
        }
        if self.0 & libc::O_NOFOLLOW != 0 {
            flags |= O_NOFOLLOW;
        }
        if self.0 & libc::O_DIRECTORY != 0 {
            flags |= O_DIRECTORY;
        }
        if self.0 & libc::O_CLOEXEC != 0 {
            flags |= O_CLOEXEC;
        }

        Ok(flags)
    }
}

//==================================================================================================
// LibcFileStatusFlags
//==================================================================================================

#[derive(Debug)]
struct LibcFileStatusFlags(libc::c_int);

impl LibcFileStatusFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn nanvix_mask() -> c_int {
        O_APPEND | O_NONBLOCK | O_SYNC
    }

    fn libc_mask() -> libc::c_int {
        libc::O_APPEND | libc::O_NONBLOCK | libc::O_SYNC | libc::O_DSYNC
    }

    fn try_from_nanvix_flags(flags: c_int) -> Result<Self, Error> {
        // Check if any unsupported flags are set.
        if flags & !Self::nanvix_mask() != 0 {
            let reason: &str = "unsupported file status flags";
            error!("do_fcntl(): {reason} (flags={flags:#x?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let mut libc_flags: libc::c_int = 0;

        if flags & O_APPEND != 0 {
            libc_flags |= libc::O_APPEND;
        }
        if flags & O_NONBLOCK != 0 {
            libc_flags |= libc::O_NONBLOCK;
        }
        if flags & O_SYNC != 0 {
            libc_flags |= libc::O_SYNC;
        }

        Ok(LibcFileStatusFlags(libc_flags))
    }

    fn try_into_nanvix_flags(&self) -> Result<c_int, Error> {
        // Check if any unsupported flags are set.
        if self.0 & !Self::libc_mask() != 0 {
            let reason: &str = "unsupported file status flags";
            error!("do_fcntl(): {reason} (flags={:#x?})", self.0);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let mut flags: c_int = 0;

        if self.0 & libc::O_APPEND != 0 {
            flags |= O_APPEND;
        }
        if self.0 & libc::O_NONBLOCK != 0 {
            flags |= O_NONBLOCK;
        }
        if self.0 & libc::O_SYNC != 0 {
            flags |= O_SYNC;
        }
        if self.0 & libc::O_DSYNC != 0 {
            let reason: &str = "O_DSYNC is not supported by Nanvix";
            error!("do_fcntl(): {reason} (flags={:#x?})", self.0);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        if self.0 & libc::O_RSYNC != 0 {
            let reason: &str = "O_RSYNC is not supported by Nanvix";
            error!("do_fcntl(): {reason} (flags={:#x?})", self.0);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(flags)
    }
}

//==================================================================================================
// LibcFileControlCommand
//==================================================================================================

#[derive(Debug)]
struct LibcFileControlCommand(libc::c_int);

impl LibcFileControlCommand {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(cmd: i32) -> Result<LibcFileControlCommand, Error> {
        let libc_cmd: libc::c_int = match cmd {
            F_DUPFD => libc::F_DUPFD,
            F_GETFD => libc::F_GETFD,
            F_SETFD => libc::F_SETFD,
            F_GETFL => libc::F_GETFL,
            F_SETFL => libc::F_SETFL,
            F_GETOWN => libc::F_GETOWN,
            F_SETOWN => libc::F_SETOWN,
            F_GETLK => libc::F_GETLK,
            F_SETLK => libc::F_SETLK,
            F_SETLKW => libc::F_SETLKW,
            F_DUPFD_CLOEXEC => libc::F_DUPFD_CLOEXEC,
            F_DUPFD_CLOFORK => {
                let reason: &str = "F_DUPFD_CLOFORK is not supported by libc";
                error!("do_fcntl(): {reason} (cmd={cmd:#x?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
            _ => {
                let reason: &str = "unsupported file control command";
                error!("do_fcntl(): {reason} (cmd={cmd:#x?})");
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            },
        };

        Ok(LibcFileControlCommand(libc_cmd))
    }
}

//==================================================================================================
// LibcFileDescriptorFlags
//==================================================================================================

struct LibcFileDescriptorFlags(libc::c_int);

impl LibcFileDescriptorFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn nanvix_mask() -> c_int {
        FD_CLOEXEC | FD_CLOFORK
    }

    fn libc_mask() -> libc::c_int {
        libc::FD_CLOEXEC
    }

    fn try_from_nanvix_flags(flags: c_int) -> Result<Self, Error> {
        // Check if any unsupported flags are set.
        if flags & !Self::nanvix_mask() != 0 {
            let reason: &str = "unsupported file descriptor flags";
            error!("do_fcntl(): {reason} (flags={flags:#x?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let mut libc_flags: libc::c_int = 0;

        if flags & FD_CLOEXEC != 0 {
            libc_flags |= libc::FD_CLOEXEC;
        }
        if flags & FD_CLOFORK != 0 {
            let reason: &str = "FD_CLOFORK is not supported by libc";
            error!("do_fcntl(): {reason} (flags={flags:#x?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        Ok(LibcFileDescriptorFlags(libc_flags))
    }

    fn try_into_nanvix_flags(&self) -> Result<c_int, Error> {
        // Check if any unsupported flags are set.
        if self.0 & !Self::libc_mask() != 0 {
            let reason: &str = "unsupported file descriptor flags";
            error!("do_fcntl(): {reason} (flags={:#x?})", self.0);
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let mut flags: c_int = 0;

        if self.0 & libc::FD_CLOEXEC != 0 {
            flags |= FD_CLOEXEC;
        }

        Ok(flags)
    }
}

//==================================================================================================

struct LibcFileMode(libc::mode_t);

impl LibcFileMode {
    fn inner(&self) -> libc::mode_t {
        self.0
    }

    fn try_from(mode: mode_t) -> Result<LibcFileMode, Error> {
        let mode_mappings: [(mode_t, u32); 12] = [
            (S_IRWXU, libc::S_IRWXU),
            (S_IRUSR, libc::S_IRUSR),
            (S_IWUSR, libc::S_IWUSR),
            (S_IXUSR, libc::S_IXUSR),
            (S_IRWXG, libc::S_IRWXG),
            (S_IRGRP, libc::S_IRGRP),
            (S_IWGRP, libc::S_IWGRP),
            (S_IXGRP, libc::S_IXGRP),
            (S_IRWXO, libc::S_IRWXO),
            (S_IROTH, libc::S_IROTH),
            (S_IWOTH, libc::S_IWOTH),
            (S_IXOTH, libc::S_IXOTH),
        ];

        // TODO: check for unsupported flags.

        let mut libc_mode: libc::mode_t = 0;
        for (nanvix_mode, m) in mode_mappings.iter() {
            if (mode & nanvix_mode) == *nanvix_mode {
                libc_mode |= *m;
            }
        }

        Ok(LibcFileMode(libc_mode))
    }
}

pub struct LibcAtFlags(libc::c_int);

impl LibcAtFlags {
    pub fn inner(&self) -> libc::c_int {
        self.0
    }

    pub fn from(flags: ffi::c_int) -> LibcAtFlags {
        let libc_flags: libc::c_int = match flags {
            AT_FDCWD => libc::AT_FDCWD,
            AT_SYMLINK_NOFOLLOW => libc::AT_SYMLINK_NOFOLLOW,
            AT_EACCESS => libc::AT_EACCESS,
            flags => flags,
        };

        LibcAtFlags(libc_flags)
    }

    /// Translates a guest `dirfd` argument of an `*at()` system call into a host
    /// `dirfd`.
    ///
    /// Unlike [`LibcAtFlags::from`], which maps individual `*at()` flag
    /// constants by exact value, this only maps the `AT_FDCWD` sentinel onto its
    /// host counterpart and forwards any real file descriptor verbatim. Routing
    /// a `dirfd` through [`LibcAtFlags::from`] would corrupt small descriptor
    /// values (e.g. `1` and `2`) that collide with the numeric values of
    /// `AT_EACCESS` and `AT_SYMLINK_NOFOLLOW`.
    pub fn from_dirfd(dirfd: ffi::c_int) -> LibcAtFlags {
        let libc_dirfd: libc::c_int = match dirfd {
            AT_FDCWD => libc::AT_FDCWD,
            dirfd => dirfd,
        };

        LibcAtFlags(libc_dirfd)
    }
}

pub struct LibcFileAdvice(libc::c_int);

impl LibcFileAdvice {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(advice: i32) -> Result<LibcFileAdvice, Error> {
        let libc_advice: libc::c_int = match advice {
            POSIX_FADV_NORMAL => libc::POSIX_FADV_NORMAL,
            POSIX_FADV_RANDOM => libc::POSIX_FADV_RANDOM,
            POSIX_FADV_SEQUENTIAL => libc::POSIX_FADV_SEQUENTIAL,
            POSIX_FADV_WILLNEED => libc::POSIX_FADV_WILLNEED,
            POSIX_FADV_DONTNEED => libc::POSIX_FADV_DONTNEED,
            POSIX_FADV_NOREUSE => libc::POSIX_FADV_NOREUSE,
            _ => return Err(Error::new(ErrorCode::InvalidArgument, "invalid advice")),
        };

        Ok(LibcFileAdvice(libc_advice))
    }
}

//==================================================================================================
// System Call Wrappers
//==================================================================================================

/// Handler for `libc::openat()`.
unsafe fn handle_openat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> libc::c_int {
    match &syscall_table.openat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, flags, mode)
        },
    }
}

/// Handler for `libc::unlinkat()`.
unsafe fn handle_unlinkat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    flags: libc::c_int,
) -> libc::c_int {
    match &syscall_table.unlinkat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, flags)
        },
    }
}

/// Handler for `libc::renameat()`.
unsafe fn handle_renameat<T>(
    syscall_table: &SyscallTable<T>,
    olddirfd: libc::c_int,
    oldpath: *const libc::c_char,
    newdirfd: libc::c_int,
    newpath: *const libc::c_char,
) -> libc::c_int {
    match &syscall_table.renameat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, olddirfd, oldpath, newdirfd, newpath)
        },
    }
}

/// Handler for `libc::fstatat()`.
unsafe fn handle_fstatat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    buf: *mut libc::stat,
    flags: libc::c_int,
) -> libc::c_int {
    match &syscall_table.fstatat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, buf, flags)
        },
    }
}

/// Handler for `libc::posix_fallocate()`.
unsafe fn handle_posix_fallocate<T>(
    syscall_table: &SyscallTable<T>,
    fd: libc::c_int,
    offset: off_t,
    len: off_t,
) -> libc::c_int {
    match &syscall_table.posix_fallocate {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, fd, offset, len)
        },
    }
}

/// Handler for `libc::posix_fadvise()`.
unsafe fn handle_posix_fadvise<T>(
    syscall_table: &SyscallTable<T>,
    fd: libc::c_int,
    offset: off_t,
    len: off_t,
    advice: libc::c_int,
) -> libc::c_int {
    match &syscall_table.posix_fadvise {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, fd, offset, len, advice)
        },
    }
}

/// Handler for `libc::fstat()`.
unsafe fn handle_fstat<T>(
    syscall_table: &SyscallTable<T>,
    fd: libc::c_int,
    buf: *mut libc::stat,
) -> libc::c_int {
    match &syscall_table.fstat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe { syscall_fn(&syscall_table.state, fd, buf) },
    }
}

/// Handler for `libc::symlinkat()`.
unsafe fn handle_symlinkat<T>(
    syscall_table: &SyscallTable<T>,
    target: *const libc::c_char,
    newdirfd: libc::c_int,
    linkpath: *const libc::c_char,
) -> libc::c_int {
    match &syscall_table.symlinkat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, target, newdirfd, linkpath)
        },
    }
}

/// Handler for `libc::readlinkat()`.
unsafe fn handle_readlinkat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    buf: *mut libc::c_char,
    bufsiz: libc::size_t,
) -> libc::ssize_t {
    match &syscall_table.readlinkat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, buf, bufsiz)
        },
    }
}

/// Handler for `libc::mkdirat()`.
unsafe fn handle_mkdirat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    mode: libc::mode_t,
) -> libc::c_int {
    match &syscall_table.mkdirat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, mode)
        },
    }
}

/// Handler for `libc::utimensat()`.
unsafe fn handle_utimensat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    times: *const libc::timespec,
    flags: libc::c_int,
) -> libc::c_int {
    match &syscall_table.utimensat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, times, flags)
        },
    }
}

/// Handler for `libc::futimens()`.
unsafe fn handle_futimens<T>(
    syscall_table: &SyscallTable<T>,
    fd: libc::c_int,
    times: *const libc::timespec,
) -> libc::c_int {
    match &syscall_table.futimens {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, fd, times)
        },
    }
}

/// Handler for `libc::fcntl()`.
unsafe fn handle_fcntl<T>(
    syscall_table: &SyscallTable<T>,
    fd: libc::c_int,
    cmd: libc::c_int,
    arg: libc::c_int,
) -> libc::c_int {
    match &syscall_table.fcntl {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, fd, cmd, arg)
        },
    }
}

/// Handler for `libc::fchownat()`.
unsafe fn handle_fchownat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    owner: libc::uid_t,
    group: libc::gid_t,
    flags: libc::c_int,
) -> libc::c_int {
    match &syscall_table.fchownat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, owner, group, flags)
        },
    }
}

/// Handler for `libc::fchmod()`.
unsafe fn handle_fchmod<T>(
    syscall_table: &SyscallTable<T>,
    fd: libc::c_int,
    mode: libc::mode_t,
) -> libc::c_int {
    match &syscall_table.fchmod {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe { syscall_fn(&syscall_table.state, fd, mode) },
    }
}

/// Handler for `libc::fchmodat()`.
unsafe fn handle_fchmodat<T>(
    syscall_table: &SyscallTable<T>,
    dirfd: libc::c_int,
    pathname: *const libc::c_char,
    mode: libc::mode_t,
    flags: libc::c_int,
) -> libc::c_int {
    match &syscall_table.fchmodat {
        SyscallAction::Block => {
            unsafe { *libc::__errno_location() = libc::EPERM };
            -1
        },
        SyscallAction::Forward(syscall_fn) => unsafe {
            syscall_fn(&syscall_table.state, dirfd, pathname, mode, flags)
        },
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::{
        LibcAtFlags,
        AT_EACCESS,
        AT_FDCWD,
        AT_SYMLINK_NOFOLLOW,
    };

    /// Verifies that `from_dirfd()` maps the `AT_FDCWD` sentinel onto its host
    /// counterpart.
    #[test]
    fn from_dirfd_maps_at_fdcwd() {
        assert_eq!(LibcAtFlags::from_dirfd(AT_FDCWD).inner(), libc::AT_FDCWD);
    }

    /// Verifies that `from_dirfd()` forwards real descriptors verbatim, including
    /// the small values `1` and `2` that collide with the numeric values of
    /// `AT_EACCESS` and `AT_SYMLINK_NOFOLLOW`.
    #[test]
    fn from_dirfd_forwards_real_descriptors_verbatim() {
        for dirfd in [AT_EACCESS, AT_SYMLINK_NOFOLLOW, 0, 3, 42, 1024] {
            assert_eq!(LibcAtFlags::from_dirfd(dirfd).inner(), dirfd);
        }
    }

    /// Guards the bug being fixed: routing a `dirfd` through the flag translator
    /// `from()` corrupts the small descriptor values `1` and `2`, whereas
    /// `from_dirfd()` preserves them.
    #[test]
    fn from_dirfd_does_not_corrupt_small_descriptors() {
        // `from()` rewrites the guest `AT_EACCESS`/`AT_SYMLINK_NOFOLLOW` values
        // (`1`/`2`) into the host `*at()` flag constants, corrupting any real
        // `dirfd` that happens to use those descriptor numbers.
        assert_eq!(LibcAtFlags::from(AT_EACCESS).inner(), libc::AT_EACCESS);
        assert_eq!(LibcAtFlags::from(AT_SYMLINK_NOFOLLOW).inner(), libc::AT_SYMLINK_NOFOLLOW);

        // `from_dirfd()` instead preserves the descriptor values verbatim.
        assert_eq!(LibcAtFlags::from_dirfd(AT_EACCESS).inner(), AT_EACCESS);
        assert_eq!(LibcAtFlags::from_dirfd(AT_SYMLINK_NOFOLLOW).inner(), AT_SYMLINK_NOFOLLOW);
    }
}
