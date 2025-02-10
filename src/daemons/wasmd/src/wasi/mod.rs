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
    engine::{
        WasiFile,
        WasiSocket,
    },
    pal::{
        fs::File,
        socket::Socket,
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

struct WasiCtxInner {
    next_wasi_fd: Fd,
    files: Vec<WasiFile>,
    preopen_dirs: Vec<(WasiFile, String)>,
    preopen_sockets: Vec<WasiSocket>,
    envs: Vec<String>,
    args: Vec<String>,
}
impl WasiCtxInner {
    /// Standard input file descriptor.
    const STDIN: Fd = 0;
    /// Standard output file descriptor.
    const STDOUT: Fd = 1;
    /// Standard error file descriptor.
    const STDERR: Fd = 2;

    pub fn new(
        next_wasi_fd: Fd,
        files: Vec<WasiFile>,
        preopen_dirs: Vec<(WasiFile, String)>,
        preopen_sockets: Vec<WasiSocket>,
        envs: Vec<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            next_wasi_fd,
            preopen_dirs,
            preopen_sockets,
            files,
            envs,
            args,
        }
    }

    fn insert_file(&mut self, os_file: File, base_rights: Rights, inherited_rights: Rights) -> Fd {
        let wasi_fd: Fd = self.next_wasi_fd;
        self.files
            .push(WasiFile::new(wasi_fd, os_file, base_rights, inherited_rights));
        self.next_wasi_fd += 1;
        wasi_fd
    }

    fn remove_file(&mut self, fd: Fd) -> Result<(), Errno> {
        // Check if we are removing stdin, stdout, or stderr.
        if fd == Self::STDIN || fd == Self::STDOUT || fd == Self::STDERR {
            ::nvx::error!("remove_file(): cannot remove stdin, stdout, nor stderr");
            return Err(Errno::Badf);
        }

        // Find and remove file descriptor from the list of open files.
        let num_open_files: usize = self.files.len();
        self.files.retain(|file| file.fd() != fd);

        // Check if file descriptor was removed.
        if self.files.len() == num_open_files {
            return Err(Errno::Badf);
        }

        debug_assert!(self.files.len() == num_open_files - 1);

        Ok(())
    }

    fn insert_socket(
        &mut self,
        socket: Socket,
        base_rights: &Rights,
        inherited_rights: &Rights,
    ) -> Fd {
        let wasi_fd: Fd = self.next_wasi_fd;
        self.preopen_sockets
            .push(WasiSocket::new(wasi_fd, socket, base_rights, inherited_rights));
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

    fn get_file_mut(&mut self, fd: Fd) -> Option<&mut WasiFile> {
        // Search for file descriptor in the list of pre-open directories.
        if let Some(file) = self
            .preopen_dirs
            .iter_mut()
            .find(|(file, _)| file.fd() == fd)
            .map(|(file, _)| file)
        {
            return Some(file);
        }

        // Search for file descriptor in the list of open files.
        self.files.iter_mut().find(|file| file.fd() == fd)
    }

    fn get_socket(&self, sockfd: Fd) -> Option<&WasiSocket> {
        self.preopen_sockets
            .iter()
            .find(|socket| socket.fd() == sockfd)
    }

    fn get_socket_mut(&mut self, sockfd: Fd) -> Option<&mut WasiSocket> {
        self.preopen_sockets
            .iter_mut()
            .find(|socket| socket.fd() == sockfd)
    }
}

pub struct WasiCtx(RefCell<WasiCtxInner>);

unsafe impl Send for WasiCtx {}
unsafe impl Sync for WasiCtx {}

impl WasiCtx {
    pub fn new(
        next_wasi_fd: Fd,
        files: Vec<WasiFile>,
        preopen_dirs: Vec<(WasiFile, String)>,
        preopen_sockets: Vec<WasiSocket>,
        envs: Vec<String>,
        args: Vec<String>,
    ) -> Self {
        Self(RefCell::new(WasiCtxInner::new(
            next_wasi_fd,
            files,
            preopen_dirs,
            preopen_sockets,
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

    /// Moves the offset of a file descriptor.
    pub fn fd_seek(&self, fd: Fd, offset: FileDelta, whence: Whence) -> Result<FileSize, Errno> {
        self.0.borrow_mut().fd_seek(fd, offset, whence)
    }

    /// Reads from a file descriptor.
    pub fn fd_read(&self, memory: &mut [u8], fd: Fd, iovecs: &[IoVec]) -> Result<Size, Errno> {
        self.0.borrow().fd_read(memory, fd, iovecs)
    }

    /// Returns the current offset of a file descriptor.
    pub fn fd_tell(&self, fd: Fd) -> Result<FileSize, Errno> {
        self.0.borrow().fd_tell(fd)
    }

    /// Writes to a file descriptor.
    pub fn fd_write(&self, memory: &[u8], fd: Fd, iovecs: &[IoVec]) -> Result<Size, Errno> {
        self.0.borrow_mut().fd_write(memory, fd, iovecs)
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

    /// Accepts an incoming connection.
    pub fn sock_accept(&self, sockfd: Fd, fdflags: FdFlags) -> Result<Fd, Errno> {
        self.0.borrow_mut().sock_accept(sockfd, fdflags)
    }

    /// Receives payload from socket connection.
    pub fn sock_recv(
        &self,
        memory: &mut [u8],
        connfd: Fd,
        iovecs: &[IoVec],
        riflags: RiFlags,
    ) -> Result<(Size, RoFlags), Errno> {
        self.0
            .borrow_mut()
            .sock_recv(memory, connfd, iovecs, riflags)
    }

    /// Sends payload to socket connection.
    pub fn sock_send(
        &self,
        memory: &[u8],
        connfd: Fd,
        iovecs: &[IoVec],
        siflags: SiFlags,
    ) -> Result<Size, Errno> {
        self.0
            .borrow_mut()
            .sock_send(memory, connfd, iovecs, siflags)
    }

    // Shutdowns send and receive operations on a socket.
    pub fn sock_shutdown(&self, sockfd: Fd, how: SdFlags) -> Result<(), Errno> {
        self.0.borrow_mut().sock_shutdown(sockfd, how)
    }
}
