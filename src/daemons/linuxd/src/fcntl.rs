// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::time::LibcTimeSpec;
use ::alloc::ffi::CString;
use ::core::ffi;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};
use ::syscall::{
    fcntl,
    fcntl::{
        message::{
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
        OpenFlags,
        AT_REMOVEDIR,
    },
    ffi::c_int,
    limits::PATH_MAX,
    message::MessagePartitioner,
    sys::{
        stat::{
            message::{
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
            stat,
        },
        types::{
            mode_t,
            off_t,
        },
    },
    time::timespec,
    unistd::message::{
        FileChownAtRequest,
        FileChownAtResponse,
        ReadLinkAtRequest,
        ReadLinkAtResponse,
        SymbolicLinkAtRequest,
        SymbolicLinkAtResponse,
    },
};

//==================================================================================================
// do_openat
//==================================================================================================

pub fn do_openat(pid: ProcessIdentifier, request: OpenAtRequest) -> Vec<Message> {
    trace!("openat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let flags: ffi::c_int = request.flags;
    let mode: mode_t = request.mode;

    let pathname: CString = match CString::new(request.pathname.as_str()) {
        Ok(pathname) => pathname,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);
    let flags: LibcFileFlags = match LibcFileFlags::try_from(flags) {
        Ok(flags) => flags,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };
    let mode: LibcFileMode = match LibcFileMode::try_from(mode) {
        Ok(mode) => mode,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    debug!(
        "libc::openat(): dirfd={:?}, pathname={pathname:?}, flags={:?}, mode={:?}",
        dirfd.inner(),
        flags.inner(),
        mode.inner()
    );
    match unsafe { libc::openat(dirfd.inner(), pathname.as_ptr(), flags.inner(), mode.inner()) } {
        fd if fd >= 0 => {
            debug!("libc::openat(): fd={fd:?}");
            vec![OpenAtResponse::build(pid, fd)]
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::openat(): errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno)
                .unwrap_or_else(|_| panic!("unknown error code {errno:?}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_unlink_at
//==================================================================================================

pub fn do_unlinkat(pid: ProcessIdentifier, request: UnlinkAtRequest) -> Vec<Message> {
    trace!("unlinkat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let flags: c_int = request.flags;

    let pathname: CString = match CString::new(request.pathname.as_str()) {
        Ok(pathname) => pathname,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);
    let flags: libc::c_int = if flags == AT_REMOVEDIR {
        libc::AT_REMOVEDIR
    } else {
        0
    };

    debug!(
        "libc::unlinkat(): dirfd={:?}, pathname={pathname:?}, flags={flags:?}",
        dirfd.inner(),
    );
    match unsafe { libc::unlinkat(dirfd.inner(), pathname.as_bytes().as_ptr() as *const i8, flags) }
    {
        ret if ret == 0 => {
            debug!("libc::unlinkat(): success");
            vec![UnlinkAtResponse::build(pid, ret)]
        },
        errno => {
            debug!("libc::unlinkat(): errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_rename_at
//==================================================================================================

pub fn do_renameat(pid: ProcessIdentifier, request: RenameAtRequest) -> Vec<Message> {
    trace!("renameat(): pid={pid:?}, request={request:?}");

    let olddirfd: i32 = request.olddirfd;
    let newdirfd: i32 = request.newdirfd;

    let oldpath: CString = match CString::new(request.oldpath.as_str()) {
        Ok(oldpath) => oldpath,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let newpath: CString = match CString::new(request.newpath.as_str()) {
        Ok(newpath) => newpath,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let olddirfd: LibcAtFlags = LibcAtFlags::from(olddirfd);
    let newdirfd: LibcAtFlags = LibcAtFlags::from(newdirfd);

    debug!(
        "libc::renameat(): olddirfd={:?}, oldpath={oldpath:?}, newdirfd={:?}, newpath={newpath:?}",
        olddirfd.inner(),
        newdirfd.inner(),
    );
    match unsafe {
        libc::renameat(
            olddirfd.inner(),
            oldpath.as_bytes().as_ptr() as *const i8,
            newdirfd.inner(),
            newpath.as_bytes().as_ptr() as *const i8,
        )
    } {
        ret if ret == 0 => {
            debug!("libc::renameat(): success");
            vec![RenameAtResponse::build(pid, ret)]
        },
        errno => {
            debug!("libc::renameat(): errno={errno:?}");
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_fstatat
//==================================================================================================

pub fn do_fstat_at(pid: ProcessIdentifier, request: FileStatAtRequest) -> Vec<Message> {
    trace!("fstatat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);
    let flag: i32 = request.flag;
    let flag: LibcFileFlags = LibcFileFlags(flag);
    let path: CString = match CString::new(request.path.as_str()) {
        Ok(c_string) => c_string,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let mut st: libc::stat = unsafe { core::mem::zeroed() };

    debug!("libc::fstatat(): dirfd={:?}, path={path:?}, flag={:?}", dirfd.inner(), flag.inner());
    match unsafe {
        libc::fstatat(dirfd.inner(), path.as_ptr(), &mut st as *mut libc::stat, flag.inner())
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
                            return vec![crate::build_error(pid, ErrorCode::ValueOutOfRange)];
                        },
                    },
                },
                st_mtim: timespec {
                    tv_sec: st.st_mtime,
                    tv_nsec: match st.st_mtime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return vec![crate::build_error(pid, ErrorCode::ValueOutOfRange)];
                        },
                    },
                },
                st_ctim: timespec {
                    tv_sec: st.st_ctime,
                    tv_nsec: match st.st_ctime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return vec![crate::build_error(pid, ErrorCode::ValueOutOfRange)];
                        },
                    },
                },
                st_blksize: st.st_blksize,
                st_blocks: st.st_blocks,
            };

            // Print size of stat structure.
            debug!("libc::fstatat(): size of stat={:?}", core::mem::size_of::<stat>());
            let response = FileStatAtResponse::new(stat);

            match response.into_parts(pid) {
                Ok(messages) => messages,
                Err(e) => vec![crate::build_error(pid, e.code)],
            }
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::fstatat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_posix_fallocate
//==================================================================================================

pub fn do_posix_fallocate(pid: ProcessIdentifier, request: FileSpaceControlRequest) -> Message {
    trace!("posix_fallocate(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let offset: off_t = request.offset;
    let len: off_t = request.len;

    debug!("libc::posix_fallocate(): fd={fd:?}, offset={offset:?}, len={len:?}");
    match unsafe { libc::posix_fallocate(fd, offset, len) } {
        0 => {
            debug!("libc::posix_fallocate(): success");
            FileSpaceControlResponse::build(pid, 0)
        },
        errno => {
            debug!("libc::posix_fallocate(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================
// do_posix_fadvise
//==================================================================================================

pub fn do_posix_fadvise(
    pid: ProcessIdentifier,
    request: FileAdvisoryInformationRequest,
) -> Message {
    trace!("posix_fadvise(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let offset: off_t = request.offset;
    let len: off_t = request.len;
    let advice: LibcFileAdvice = match LibcFileAdvice::try_from(request.advice) {
        Ok(advice) => advice,
        Err(e) => return crate::build_error(pid, e.code),
    };

    debug!(
        "libc::posix_fadvise(): fd={fd:?}, offset={offset:?}, len={len:?}, advice={:?}",
        advice.inner()
    );
    match unsafe { libc::posix_fadvise(fd, offset, len, advice.inner()) } {
        0 => {
            debug!("libc::posix_fadvise(): success");
            FileAdvisoryInformationResponse::build(pid, 0)
        },
        errno => {
            debug!("libc::posix_fadvise(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================
// do_fstat()
//==================================================================================================

pub fn do_fstat(pid: ProcessIdentifier, request: FileStatRequest) -> Vec<Message> {
    trace!("fstatat(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;

    let mut st: libc::stat = unsafe { core::mem::zeroed() };

    debug!("libc::fstat(): fd={fd:?}");
    match unsafe { libc::fstat(fd, &mut st) } {
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
                            return vec![crate::build_error(pid, ErrorCode::ValueOutOfRange)];
                        },
                    },
                },
                st_mtim: timespec {
                    tv_sec: st.st_mtime,
                    tv_nsec: match st.st_mtime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return vec![crate::build_error(pid, ErrorCode::ValueOutOfRange)];
                        },
                    },
                },
                st_ctim: timespec {
                    tv_sec: st.st_ctime,
                    tv_nsec: match st.st_ctime_nsec.try_into() {
                        Ok(nsec) => nsec,
                        Err(_) => {
                            return vec![crate::build_error(pid, ErrorCode::ValueOutOfRange)];
                        },
                    },
                },
                st_blksize: st.st_blksize,
                st_blocks: st.st_blocks,
            };

            // Print size of stat structure.
            debug!("libc::fstatat(): size of stat={:?}", core::mem::size_of::<stat>());
            let response = FileStatAtResponse::new(stat);

            match response.into_parts(pid) {
                Ok(messages) => messages,
                Err(e) => vec![crate::build_error(pid, e.code)],
            }
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::fstatat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_symlinkat()
//==================================================================================================

pub fn do_symlinkat(pid: ProcessIdentifier, request: SymbolicLinkAtRequest) -> Vec<Message> {
    trace!("symlinkat(): pid={pid:?}, request={request:?}");

    let target: CString = match CString::new(request.target.as_str()) {
        Ok(target) => target,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let newdirfd: i32 = request.dirfd;
    let newdirfd: LibcAtFlags = LibcAtFlags::from(newdirfd);

    let linkpath: CString = match CString::new(request.linkpath.as_str()) {
        Ok(linkpath) => linkpath,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    debug!(
        "libc::symlinkat(): oldpath={target:?}, newdirfd={:?}, newpath={linkpath:?}",
        newdirfd.inner(),
    );
    match unsafe { libc::symlinkat(target.as_ptr(), newdirfd.inner(), linkpath.as_ptr()) } {
        0 => {
            debug!("libc::symlinkat(): success");
            vec![SymbolicLinkAtResponse::build(pid, 0)]
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::symlinkat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_readlinkat()
//==================================================================================================

pub fn do_readlinkat(pid: ProcessIdentifier, request: ReadLinkAtRequest) -> Vec<Message> {
    trace!("readlinkat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    // TODO: Have a system-wide constant for this.
    let mut buf: Vec<u8> = vec![0u8; PATH_MAX];

    debug!(
        "libc::readlinkat(): dirfd={:?}, path={path:?}, capacity={:?}",
        dirfd.inner(),
        buf.capacity()
    );
    match unsafe {
        libc::readlinkat(dirfd.inner(), path.as_ptr(), buf.as_mut_ptr() as *mut i8, buf.capacity())
    } {
        len if len >= 0 => {
            debug!("libc::readlinkat(): (len={len:?})");

            buf.truncate(len as usize);

            let response: ReadLinkAtResponse = match ReadLinkAtResponse::new(buf) {
                Ok(response) => response,
                Err(e) => return vec![crate::build_error(pid, e.code)],
            };

            match response.into_parts(pid) {
                Ok(messages) => messages,
                Err(e) => vec![crate::build_error(pid, e.code)],
            }
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::readlinkat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_mkdirat()
//==================================================================================================

pub fn do_mkdirat(pid: ProcessIdentifier, request: MakeDirectoryAtRequest) -> Vec<Message> {
    trace!("mkdirat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);

    let pathname: CString = match CString::new(request.pathname.as_str()) {
        Ok(pathname) => pathname,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let mode: LibcFileMode = match LibcFileMode::try_from(request.mode) {
        Ok(mode) => mode,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    debug!(
        "libc::mkdirat(): dirfd={:?}, pathname={pathname:?}, mode={:?}",
        dirfd.inner(),
        mode.inner()
    );
    match unsafe { libc::mkdirat(dirfd.inner(), pathname.as_ptr(), mode.inner()) } {
        0 => {
            debug!("libc::mkdirat(): success");
            vec![MakeDirectoryAtResponse::build(pid, 0)]
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::mkdirat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_utimensat()
//==================================================================================================

pub fn do_utimensat(
    pid: ProcessIdentifier,
    request: UpdateFileAccessTimeAtRequest,
) -> Vec<Message> {
    trace!("utimensat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let times: [timespec; 2] = request.times;

    let libc_times: [libc::timespec; 2] = [
        Into::<LibcTimeSpec>::into(times[0]).into(),
        Into::<LibcTimeSpec>::into(times[1]).into(),
    ];

    let flag: LibcFileFlags = LibcFileFlags(request.flag);

    debug!(
        "libc::utimensat(): dirfd={:?}, path={path:?}, flag={:?}, times[0].tv_sec={:?}, \
         times[0].tv_nsec={:?}, times[1].tv_sec={:?}, times[1].tv_nsec={:?}",
        dirfd.inner(),
        flag.inner(),
        libc_times[0].tv_sec,
        libc_times[0].tv_nsec,
        libc_times[1].tv_sec,
        libc_times[1].tv_nsec
    );
    match unsafe {
        libc::utimensat(dirfd.inner(), path.as_ptr(), libc_times.as_ptr(), flag.inner())
    } {
        0 => {
            debug!("libc::utimensat(): success");
            vec![UpdateFileAccessTimeAtResponse::build(pid, 0)]
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::utimensat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_futimens()
//==================================================================================================

pub fn do_futimens(pid: ProcessIdentifier, request: UpdateFileAccessTimeRequest) -> Message {
    trace!("futimens(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;

    let times: [timespec; 2] = request.times;

    let libc_times: [libc::timespec; 2] = [
        Into::<LibcTimeSpec>::into(times[0]).into(),
        Into::<LibcTimeSpec>::into(times[1]).into(),
    ];

    debug!(
        "libc::futimens(): fd={fd:?}, times[0].tv_sec={:?}, times[0].tv_nsec={:?}, \
         times[1].tv_sec={:?}, times[1].tv_nsec={:?}",
        libc_times[0].tv_sec,
        libc_times[0].tv_nsec,
        libc_times[1].tv_sec,
        libc_times[1].tv_nsec
    );
    match unsafe { libc::futimens(fd, libc_times.as_ptr()) } {
        0 => {
            debug!("libc::futimens(): success");
            UpdateFileAccessTimeResponse::build(pid, 0)
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::futimens(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================
// do_fcntl()
//==================================================================================================

pub fn do_fcntl(pid: ProcessIdentifier, request: FileControlRequest) -> Message {
    trace!("fcntl(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let cmd: LibcFileControlCommand = match LibcFileControlCommand::try_from(request.cmd) {
        Ok(cmd) => cmd,
        Err(e) => return crate::build_error(pid, e.code),
    };
    let arg: u32 = request.arg;

    debug!("libc::fcntl(): fd={fd:?}, cmd={:?}, arg={arg:?}", cmd.inner());

    let ret: i32 = unsafe { libc::fcntl(fd, cmd.inner(), arg) };

    match cmd.inner() {
        libc::F_GETFD => {
            if ret >= 0 {
                debug!("libc::fcntl(): F_GETFD success");
                todo!("convert file descriptor flags");
            } else if ret == -1 {
                let errno: i32 = unsafe { *libc::__errno_location() };
                debug!("libc::fcntl(): errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("unknown error code {errno}"));
                crate::build_error(pid, error)
            } else {
                unreachable!("should not return -1")
            }
        },
        libc::F_GETFL => {
            if ret >= 0 {
                debug!("libc::fcntl(): F_GETFL success");
                let flags: i32 = match LibcFileFlags::try_from(ret) {
                    Ok(flags) => flags.as_nanvix_flags(),
                    Err(e) => return crate::build_error(pid, e.code),
                };
                FileControlResponse::build(pid, flags)
            } else if ret == -1 {
                let errno: i32 = unsafe { *libc::__errno_location() };
                debug!("libc::fcntl(): errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("unknown error code {errno}"));
                crate::build_error(pid, error)
            } else {
                unreachable!("should not fail with a value other than -1")
            }
        },
        libc::F_GETOWN => {
            if ret >= 0 || ret != -1 {
                debug!("libc::fcntl(): F_GETOWN success");
                FileControlResponse::build(pid, ret)
            } else {
                unreachable!("should not return -1");
            }
        },
        libc::F_SETFD | libc::F_SETFL | libc::F_SETOWN => {
            if ret == 0 {
                debug!("libc::fcntl(): success");
                FileControlResponse::build(pid, ret)
            } else if ret == -1 {
                let errno: i32 = unsafe { *libc::__errno_location() };
                debug!("libc::fcntl(): errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("unknown error code {errno}"));
                crate::build_error(pid, error)
            } else {
                unreachable!("should not fail with a value other than -1")
            }
        },
        libc::F_DUPFD | libc::F_DUPFD_CLOEXEC => {
            if ret >= 0 {
                debug!("libc::fcntl(): success");
                FileControlResponse::build(pid, ret)
            } else {
                unreachable!("should not return a negative value");
            }
        },
        _ => {
            unreachable!("unsupported command");
        },
    }
}

//==================================================================================================
// do_fchownat()
//==================================================================================================

pub fn do_fchownat(pid: ProcessIdentifier, request: FileChownAtRequest) -> Vec<Message> {
    trace!("fchownat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let owner: u32 = request.owner;
    let group: u32 = request.group;
    let flag: LibcAtFlags = LibcAtFlags::from(request.flag);

    debug!(
        "libc::fchownat(): dirfd={:?}, path={path:?}, owner={owner:?}, group={group:?}, flag={:?}",
        dirfd.inner(),
        flag.inner()
    );
    match unsafe { libc::fchownat(dirfd.inner(), path.as_ptr(), owner, group, flag.inner()) } {
        0 => {
            debug!("libc::fchownat(): success");
            vec![FileChownAtResponse::build(pid)]
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::fchownat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================
// do_fchmod
//==================================================================================================

pub fn do_fchmod(pid: ProcessIdentifier, request: FileChmodRequest) -> Message {
    trace!("fchmod(): pid={pid:?}, request={request:?}");

    let fd: i32 = request.fd;
    let mode: u32 = request.mode;

    debug!("libc::fchmod(): fd={fd:?}, mode={mode:?}");
    match unsafe { libc::fchmod(fd, mode) } {
        0 => FileChmodResponse::build(pid),
        ret if ret == -1 => {
            let errno: libc::c_int = unsafe { *libc::__errno_location() };
            crate::build_error(
                pid,
                ErrorCode::try_from(errno)
                    .unwrap_or_else(|_| panic!("invalid error code: {ret:?}")),
            )
        },
        ret => unreachable!("libc::fchmod() returned an invalid value ({ret:?})"),
    }
}

//==================================================================================================
// do_fchmodat()
//==================================================================================================

pub fn do_fchmodat(pid: ProcessIdentifier, request: FileChmodAtRequest) -> Vec<Message> {
    trace!("fchmodat(): pid={pid:?}, request={request:?}");

    let dirfd: i32 = request.dirfd;
    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);

    let path: CString = match CString::new(request.path.as_str()) {
        Ok(path) => path,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let mode: LibcFileMode = match LibcFileMode::try_from(request.mode) {
        Ok(mode) => mode,
        Err(_) => return vec![crate::build_error(pid, ErrorCode::InvalidMessage)],
    };

    let flag: LibcAtFlags = LibcAtFlags::from(request.flag);

    debug!(
        "libc::fchmodat(): dirfd={:?}, path={:?}, mode={:?}, flags={:?}",
        dirfd.inner(),
        path,
        mode.inner(),
        flag.inner()
    );
    match unsafe { libc::fchmodat(dirfd.inner(), path.as_ptr(), mode.inner(), flag.inner()) } {
        0 => {
            debug!("libc::fchmodat(): success");
            vec![FileChmodAtResponse::build(pid)]
        },
        _ => {
            let errno: i32 = unsafe { *libc::__errno_location() };
            debug!("libc::fchmodat(): errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(errno).unwrap_or_else(|_| panic!("unknown error code {errno}"));
            vec![crate::build_error(pid, error)]
        },
    }
}

//==================================================================================================

struct LibcFileFlags(libc::c_int);

impl LibcFileFlags {
    const FLAG_MAPPINGS: [(OpenFlags, ffi::c_int); 5] = [
        (fcntl::OpenFlags::O_APPEND, libc::O_APPEND),
        (fcntl::OpenFlags::O_CREAT, libc::O_CREAT),
        (fcntl::OpenFlags::O_EXCL, libc::O_EXCL),
        (fcntl::OpenFlags::O_TRUNC, libc::O_TRUNC),
        (fcntl::OpenFlags::O_DIRECTORY, libc::O_DIRECTORY),
    ];

    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(flags: ffi::c_int) -> Result<LibcFileFlags, Error> {
        // TODO: check for unsupported flags.
        let mut libc_flags: libc::c_int = 0;

        // Set access mode.
        if flags & fcntl::O_ACCMODE == fcntl::OpenFlags::O_RDONLY.into() {
            libc_flags |= libc::O_RDONLY;
        } else if flags & fcntl::O_ACCMODE == fcntl::OpenFlags::O_WRONLY.into() {
            libc_flags |= libc::O_WRONLY;
        } else if flags & fcntl::O_ACCMODE == fcntl::OpenFlags::O_RDWR.into() {
            libc_flags |= libc::O_RDWR;
        }

        for (nanvix_flag, f) in Self::FLAG_MAPPINGS.iter() {
            if (flags & nanvix_flag) == nanvix_flag.into() {
                libc_flags |= *f;
            }
        }

        Ok(LibcFileFlags(libc_flags))
    }

    fn as_nanvix_flags(&self) -> i32 {
        let mut flags: i32 = 0;

        // Set access mode.
        match self.0 & libc::O_ACCMODE {
            libc::O_RDONLY => flags |= fcntl::OpenFlags::O_RDONLY,
            libc::O_WRONLY => flags |= fcntl::OpenFlags::O_WRONLY,
            libc::O_RDWR => flags |= fcntl::OpenFlags::O_RDWR,
            _ => {},
        }

        for (nanvix_flag, f) in Self::FLAG_MAPPINGS.iter() {
            if (self.0 & f) == *f {
                flags |= *nanvix_flag;
            }
        }

        flags
    }
}

struct LibcFileMode(libc::mode_t);

impl LibcFileMode {
    fn inner(&self) -> libc::mode_t {
        self.0
    }

    fn try_from(mode: mode_t) -> Result<LibcFileMode, Error> {
        let mode_mappings: [(mode_t, u32); 12] = [
            (fcntl::S_IRWXU, libc::S_IRWXU),
            (fcntl::S_IRUSR, libc::S_IRUSR),
            (fcntl::S_IWUSR, libc::S_IWUSR),
            (fcntl::S_IXUSR, libc::S_IXUSR),
            (fcntl::S_IRWXG, libc::S_IRWXG),
            (fcntl::S_IRGRP, libc::S_IRGRP),
            (fcntl::S_IWGRP, libc::S_IWGRP),
            (fcntl::S_IXGRP, libc::S_IXGRP),
            (fcntl::S_IRWXO, libc::S_IRWXO),
            (fcntl::S_IROTH, libc::S_IROTH),
            (fcntl::S_IWOTH, libc::S_IWOTH),
            (fcntl::S_IXOTH, libc::S_IXOTH),
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
            fcntl::AT_FDCWD => libc::AT_FDCWD,
            fcntl::AT_SYMLINK_NOFOLLOW => libc::AT_SYMLINK_NOFOLLOW,
            fcntl::AT_EACCESS => libc::AT_EACCESS,
            flags => flags,
        };

        LibcAtFlags(libc_flags)
    }
}

pub struct LibcFileAdvice(libc::c_int);

impl LibcFileAdvice {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(advice: i32) -> Result<LibcFileAdvice, Error> {
        let libc_advice: libc::c_int = match advice {
            fcntl::POSIX_FADV_NORMAL => libc::POSIX_FADV_NORMAL,
            fcntl::POSIX_FADV_RANDOM => libc::POSIX_FADV_RANDOM,
            fcntl::POSIX_FADV_SEQUENTIAL => libc::POSIX_FADV_SEQUENTIAL,
            fcntl::POSIX_FADV_WILLNEED => libc::POSIX_FADV_WILLNEED,
            fcntl::POSIX_FADV_DONTNEED => libc::POSIX_FADV_DONTNEED,
            fcntl::POSIX_FADV_NOREUSE => libc::POSIX_FADV_NOREUSE,
            _ => return Err(Error::new(ErrorCode::InvalidArgument, "invalid advice")),
        };

        Ok(LibcFileAdvice(libc_advice))
    }
}

struct LibcFileControlCommand(libc::c_int);

impl LibcFileControlCommand {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(cmd: i32) -> Result<LibcFileControlCommand, Error> {
        let libc_cmd: libc::c_int = match cmd {
            fcntl::F_DUPFD => libc::F_DUPFD,
            fcntl::F_DUPFD_CLOEXEC => libc::F_DUPFD_CLOEXEC,
            fcntl::F_GETFD => libc::F_GETFD,
            fcntl::F_SETFD => libc::F_SETFD,
            fcntl::F_GETFL => libc::F_GETFL,
            fcntl::F_SETFL => libc::F_SETFL,
            fcntl::F_GETOWN => libc::F_GETOWN,
            fcntl::F_SETOWN => libc::F_SETOWN,
            _ => return Err(Error::new(ErrorCode::InvalidArgument, "invalid command")),
        };

        Ok(LibcFileControlCommand(libc_cmd))
    }
}
