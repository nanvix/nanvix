// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    ffi,
    str,
};
use ::linuxd::{
    fcntl,
    fcntl::message::{
        OpenAtRequest,
        OpenAtResponse,
        RenameAtRequest,
        RenameAtResponse,
        UnlinkAtRequest,
        UnlinkAtResponse,
    },
    sys::types::mode_t,
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
// do_openat
//==================================================================================================

pub fn do_open_at(pid: ProcessIdentifier, request: OpenAtRequest) -> Message {
    trace!("openat(): pid={:?}, request={:?}", pid, request);

    let dirfd: i32 = request.dirfd;
    let flags: ffi::c_int = request.flags;
    let mode: mode_t = request.mode;

    let pathname: &str = match str::from_utf8(&request.pathname) {
        Ok(pathname) => pathname,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);
    let flags: LibcFileFlags = match LibcFileFlags::try_from(flags) {
        Ok(flags) => flags,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };
    let mode: LibcFileMode = match LibcFileMode::try_from(mode) {
        Ok(mode) => mode,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    debug!(
        "libc::openat(): dirfd={:?}, pathname={:?}, flags={:?}, mode={:?}",
        dirfd.inner(),
        pathname,
        flags.inner(),
        mode.inner()
    );
    match unsafe {
        libc::openat(
            dirfd.inner(),
            pathname.as_bytes().as_ptr() as *const i8,
            flags.inner(),
            mode.inner(),
        )
    } {
        fd if fd >= 0 => {
            debug!("libc::openat(): fd={:?}", fd);
            OpenAtResponse::build(pid, fd)
        },
        errno => {
            debug!("libc::openat(): errno={:?}", errno);
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================
// do_unlink_at
//==================================================================================================

pub fn do_unlink_at(pid: ProcessIdentifier, request: UnlinkAtRequest) -> Message {
    trace!("unlinkat(): pid={:?}, request={:?}", pid, request);

    let dirfd: i32 = request.dirfd;
    let flags: ffi::c_int = request.flags;

    let pathname: &str = match str::from_utf8(&request.pathname) {
        Ok(pathname) => pathname,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    let dirfd: LibcAtFlags = LibcAtFlags::from(dirfd);
    let flags: LibcFileFlags = match LibcFileFlags::try_from(flags) {
        Ok(flags) => flags,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    debug!(
        "libc::unlinkat(): dirfd={:?}, pathname={:?}, flags={:?}",
        dirfd.inner(),
        pathname,
        flags.inner()
    );
    match unsafe {
        libc::unlinkat(dirfd.inner(), pathname.as_bytes().as_ptr() as *const i8, flags.inner())
    } {
        ret if ret == 0 => {
            debug!("libc::unlinkat(): success");
            UnlinkAtResponse::build(pid, ret)
        },
        errno => {
            debug!("libc::unlinkat(): errno={:?}", errno);
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================
// do_rename_at
//==================================================================================================

pub fn do_rename_at(pid: ProcessIdentifier, request: RenameAtRequest) -> Message {
    trace!("renameat(): pid={:?}, request={:?}", pid, request);

    let olddirfd: i32 = request.olddirfd;
    let newdirfd: i32 = request.newdirfd;

    let oldpath: &str = match str::from_utf8(&request.oldpath) {
        Ok(oldpath) => oldpath,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    let newpath: &str = match str::from_utf8(&request.newpath) {
        Ok(newpath) => newpath,
        Err(_) => return crate::build_error(pid, ErrorCode::InvalidMessage),
    };

    let olddirfd: LibcAtFlags = LibcAtFlags::from(olddirfd);
    let newdirfd: LibcAtFlags = LibcAtFlags::from(newdirfd);

    debug!(
        "libc::renameat(): olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}",
        olddirfd.inner(),
        oldpath,
        newdirfd.inner(),
        newpath
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
            RenameAtResponse::build(pid, ret)
        },
        errno => {
            debug!("libc::renameat(): errno={:?}", errno);
            let error: ErrorCode = ErrorCode::try_from(errno).expect("unknown error code {error}");
            crate::build_error(pid, error)
        },
    }
}

//==================================================================================================

struct LibcFileFlags(libc::c_int);

impl LibcFileFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn try_from(flags: ffi::c_int) -> Result<LibcFileFlags, Error> {
        let flag_mappings: [(ffi::c_int, i32); 7] = [
            (fcntl::O_APPEND, libc::O_APPEND),
            (fcntl::O_CREAT, libc::O_CREAT),
            (fcntl::O_EXCL, libc::O_EXCL),
            (fcntl::O_RDONLY, libc::O_RDONLY),
            (fcntl::O_RDWR, libc::O_RDWR),
            (fcntl::O_TRUNC, libc::O_TRUNC),
            (fcntl::O_WRONLY, libc::O_WRONLY),
        ];

        // TODO: check for unsupported flags.

        let mut libc_flags: libc::c_int = 0;
        for (nanvix_flag, f) in flag_mappings.iter() {
            if (flags & nanvix_flag) == *nanvix_flag {
                libc_flags |= *f;
            }
        }

        Ok(LibcFileFlags(libc_flags))
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

struct LibcAtFlags(libc::c_int);

impl LibcAtFlags {
    fn inner(&self) -> libc::c_int {
        self.0
    }

    fn from(flags: ffi::c_int) -> LibcAtFlags {
        let libc_flags: libc::c_int = match flags {
            fcntl::AT_FDCWD => libc::AT_FDCWD,
            fcntl::AT_REMOVEDIR => libc::AT_REMOVEDIR,
            _ => flags,
        };

        LibcAtFlags(libc_flags)
    }
}
