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
pub mod fd;
pub mod path;
pub mod sock;
pub mod types;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    engine::WasiFile,
    pal::fs::File,
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

struct WasiCtxInner {
    next_wasi_fd: Fd,
    stdin: WasiFile,
    stdout: WasiFile,
    stderr: WasiFile,
    files: Vec<WasiFile>,
    preopen_dirs: Vec<(WasiFile, String)>,
    envs: Vec<String>,
    args: Vec<String>,
}
impl WasiCtxInner {
    pub fn new(
        next_wasi_fd: Fd,
        stdin: WasiFile,
        stdout: WasiFile,
        stderr: WasiFile,
        preopen_dirs: Vec<(WasiFile, String)>,
        envs: Vec<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            next_wasi_fd,
            stdin,
            stdout,
            stderr,
            preopen_dirs,
            files: Vec::new(),
            envs,
            args,
        }
    }

    fn insert_file(&mut self, os_file: File, base_rights: Rights, inherited_rights: Rights) -> Fd {
        let wasi_fd: Fd = self.next_wasi_fd;
        self.files
            .push(WasiFile::new(wasi_fd, Some(os_file), base_rights, inherited_rights));
        self.next_wasi_fd += 1;
        wasi_fd
    }

    fn get_file(&self, fd: Fd) -> Option<&WasiFile> {
        // Search for file descriptor in the list of pre-open directories.
        if let Some(file) = self
            .preopen_dirs
            .iter()
            .find(|(file, _)| file.fd() == fd)
            .map(|(file, _)| file)
        {
            return Some(file);
        }

        // Search for file descriptor in the list of open files.
        self.files.iter().find(|file| file.fd() == fd)
    }
}

pub struct WasiCtx(RefCell<WasiCtxInner>);

unsafe impl Send for WasiCtx {}
unsafe impl Sync for WasiCtx {}

impl WasiCtx {
    pub fn new(
        next_wasi_fd: Fd,
        stdin: WasiFile,
        stdout: WasiFile,
        stderr: WasiFile,
        preopen_dirs: Vec<(WasiFile, String)>,
        envs: Vec<String>,
        args: Vec<String>,
    ) -> Self {
        Self(RefCell::new(WasiCtxInner::new(
            next_wasi_fd,
            stdin,
            stdout,
            stderr,
            preopen_dirs,
            envs,
            args,
        )))
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

    /// Closes a file descriptor.
    pub fn fd_close(&self, fd: Fd) -> Result<(), Errno> {
        self.0.borrow_mut().fd_close(fd)
    }

    // Returns a description of the given pre-opened directory.
    pub fn fd_prestat_dir_get(&self, fd: Fd) -> Result<String, Errno> {
        self.0.borrow().fd_prestat_dir_get(fd)
    }

    /// Returns a description of the given pre-opened file descriptor.
    pub fn fd_prestat_get(&self, fd: Fd) -> Result<Prestat, Errno> {
        self.0.borrow().fd_prestat_get(fd)
    }

    /// Opens a file or a directory.
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

    /// Accepts a new incoming connection on a socket.
    pub fn sock_accept(&self, sockfd: Fd, fdflags: FdFlags) -> Result<Fd, Errno> {
        self.0.borrow().sock_accept(sockfd, fdflags)
    }
}
