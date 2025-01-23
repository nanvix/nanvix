// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(dead_code)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod environ;
pub mod types;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pal::{
        self,
        Descriptor,
    },
    wasi::types::Errno,
};
use ::alloc::{
    string::String,
    vec::Vec,
};
use ::core::cell::RefCell;

//==================================================================================================
// Exports
//==================================================================================================

pub use types::*;

//==================================================================================================

pub struct PreopenedDirectory {
    descriptor: Descriptor,
    path: String,
}

impl PreopenedDirectory {
    pub fn new(fd: Descriptor, path: String) -> Self {
        Self {
            descriptor: fd,
            path,
        }
    }
}

struct WasiCtxInner {
    stdin: Descriptor,
    stdout: Descriptor,
    stderr: Descriptor,
    files: Vec<Descriptor>,
    preopen_dirs: Vec<PreopenedDirectory>,
    envs: Vec<String>,
    args: Vec<String>,
}
impl WasiCtxInner {
    pub fn new(
        stdin: Descriptor,
        stdout: Descriptor,
        stderr: Descriptor,
        preopen_dirs: Vec<PreopenedDirectory>,
        envs: Vec<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            stdin,
            stdout,
            stderr,
            preopen_dirs,
            files: Vec::new(),
            envs,
            args,
        }
    }

    /// Returns a description of the given pre-opened capability.
    pub fn fd_prestat_get(&self, fd: Fd) -> Result<Prestat, Errno> {
        // Search for file descriptor in the list of pre-open directories.
        for dir in &self.preopen_dirs {
            match dir.descriptor.rawfd() == fd {
                true => {
                    return Ok(Prestat::new(dir.path.len().into()));
                },
                false => (),
            }
        }

        ::nvx::log!("fd_prestat_get(): invalid file descriptor");
        Err(Errno::Badf)
    }

    // Returns a description of the given pre-opened directory.
    pub fn fd_prestat_dir_get(&self, fd: Fd) -> Result<String, Errno> {
        // Search for file descriptor in the list of pre-open directories.
        for dir in &self.preopen_dirs {
            match dir.descriptor.rawfd() == fd {
                true => return Ok(dir.path.clone()),
                false => (),
            }
        }

        ::nvx::log!("fd_prestat_dir_get(): invalid file descriptor");
        Err(Errno::Badf)
    }

    fn get_fd(&self, fd: Fd) -> Option<&Descriptor> {
        // Search for file descriptor in the list of pre-open directories.
        for dir in &self.preopen_dirs {
            match &dir.descriptor {
                lookup_fd if lookup_fd.rawfd() == fd => return Some(&dir.descriptor),
                _ => (),
            }
        }

        // Search for file descriptor in the list of files.
        for file in &self.files {
            if file.rawfd() == fd {
                return Some(file);
            }
        }

        None
    }

    /// Opens a file or a directory.
    pub fn path_open(
        &mut self,
        fd: Fd,
        dirflags: &LookupFlags,
        pathname: &str,
        oflags: OpenFlags,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
        fdflags: FdFlags,
    ) -> Result<Fd, Errno> {
        ::nvx::log!(
            "path_open(): fd={:?}, dirflags={:#?}, pathname={:?}, oflags={:#?}, \
             fs_rights_base={:#?}, fs_rights_inheriting={:#?}, fdflags={:#?}",
            fd,
            dirflags,
            pathname,
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fdflags
        );
        match self.get_fd(fd) {
            Some(dirfd) => {
                let mut flags: pal::OpenFlags = pal::OpenFlags::empty();

                // Set creation flag.
                if oflags.creat {
                    flags.non_exclusive.create = true;
                }

                // Set directory flag.
                if oflags.directory {
                    flags.non_exclusive.directory = true;
                }

                // Set exclusive flag.
                if oflags.excl {
                    flags.non_exclusive.exclusive = true;
                }

                // Set truncation flag.
                if oflags.trunc {
                    flags.non_exclusive.truncate = true;
                }

                // Set append mode.
                if fdflags.append {
                    flags.non_exclusive.append = true;
                }

                // Set dsync mode.
                if fdflags.dsync {
                    flags.non_exclusive.dsync = true;
                }

                // Set non-blocking mode.
                if fdflags.nonblock {
                    flags.non_exclusive.non_block = true;
                }

                // Set rsync mode.
                if fdflags.rsync {
                    flags.non_exclusive.rsync = true;
                }

                // Set sync mode.
                if fdflags.sync {
                    flags.non_exclusive.sync = true;
                }

                // Set symlink follow flag.
                if !dirflags.symlink_follow {
                    flags.non_exclusive.no_follow = true;
                }

                let mode: pal::AccessMode = pal::AccessMode::default();
                match dirfd.openat(pathname, &flags, &mode) {
                    Ok(fd) => {
                        let rawfd = fd.rawfd();
                        self.files.push(fd);
                        Ok(rawfd)
                    },
                    Err(e) => Err(Errno::try_from(e as u16).unwrap()),
                }
            },
            _ => {
                ::nvx::log!("path_open(): invalid file descriptor");
                Err(Errno::Badf)
            },
        }
    }
}

pub struct WasiCtx(RefCell<WasiCtxInner>);

unsafe impl Send for WasiCtx {}
unsafe impl Sync for WasiCtx {}

impl WasiCtx {
    pub fn new(
        stdin: Descriptor,
        stdout: Descriptor,
        stderr: Descriptor,
        preopen_dirs: Vec<PreopenedDirectory>,
        envs: Vec<String>,
        args: Vec<String>,
    ) -> Self {
        Self(RefCell::new(WasiCtxInner::new(stdin, stdout, stderr, preopen_dirs, envs, args)))
    }

    /// Reads command-line argument data.
    pub fn args_get(&self) -> Result<Vec<String>, Errno> {
        self.0.borrow().args_get()
    }

    /// Returns command-line argument data sizes.
    pub fn args_sizes_get(&self) -> Result<(Size, Size), Errno> {
        self.0.borrow().args_sizes_get()
    }

    /// Read environment variable data.
    pub fn environ_get(&self) -> Result<Vec<String>, Errno> {
        self.0.borrow().environ_get()
    }

    /// Returns environment variable data sizes.
    pub fn environ_sizes_get(&self) -> Result<(Size, Size), Errno> {
        self.0.borrow().environ_sizes_get()
    }

    pub fn fd_prestat_get(&self, fd: Fd) -> Result<Prestat, Errno> {
        self.0.borrow().fd_prestat_get(fd)
    }

    pub fn fd_prestat_dir_get(&self, fd: Fd) -> Result<String, Errno> {
        self.0.borrow().fd_prestat_dir_get(fd)
    }

    pub fn path_open(
        &self,
        fd: Fd,
        dirflags: &LookupFlags,
        pathname: &str,
        oflags: OpenFlags,
        fs_rights_base: Rights,
        fs_rights_inheriting: Rights,
        fdflags: FdFlags,
    ) -> Result<Fd, Errno> {
        self.0.borrow_mut().path_open(
            fd,
            dirflags,
            pathname,
            oflags,
            fs_rights_base,
            fs_rights_inheriting,
            fdflags,
        )
    }
}
