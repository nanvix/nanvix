// Copyright(c) The Maintainers of Nanvix.
// Licensed un

//==================================================================================================
// Imports
//==================================================================================================

use crate::pal::{
    Error,
    RawFd,
};
use ::alloc::string::{
    String,
    ToString,
};
use ::nvx::sys::error::ErrorCode;
use ::posix::{
    errno,
    fcntl,
    ffi::c_int,
    sys::types::{
        mode_t,
        off_t,
    },
    unistd,
};

//==================================================================================================
// Fd
//==================================================================================================

pub struct Fd(RawFd);

//==================================================================================================
// Path
//==================================================================================================

/// A type representing a filesystem path.
pub struct Path {
    name: String,
}

impl Path {
    /// Creates a new path from a string.
    pub fn new(name: &str) -> Path {
        Path {
            name: name.to_string(),
        }
    }
}

//==================================================================================================
// File Offset
//==================================================================================================

/// A type representing an offset in a file.
pub struct FileOffset(off_t);

impl FileOffset {
    pub fn value(&self) -> off_t {
        self.0
    }
}

impl From<off_t> for FileOffset {
    fn from(offset: off_t) -> FileOffset {
        FileOffset(offset)
    }
}

//==================================================================================================
// File Whence
//==================================================================================================

/// Used for representing the position relative to which to set the offset of a file.A
#[repr(i32)]
pub enum FileWhence {
    /// The offset is set to `offset`.
    Set = unistd::SEEK_SET,
    /// The offset is set to its current location plus `offset`.
    Cur = unistd::SEEK_CUR,
    /// The offset is set to the end of the file plus `offset`.
    End = unistd::SEEK_END,
}

//==================================================================================================
// File
//==================================================================================================

/// An object providing access to an open file on the filesystem.
pub struct File {
    rawfd: Fd,
}

impl File {
    pub fn stdin() -> File {
        File {
            rawfd: Fd(unistd::STDIN_FILENO),
        }
    }

    pub fn stdout() -> File {
        File {
            rawfd: Fd(unistd::STDOUT_FILENO),
        }
    }

    pub fn stderr() -> File {
        File {
            rawfd: Fd(unistd::STDERR_FILENO),
        }
    }

    /// Opens a file in read-only mode.
    pub fn open(path: &Path) -> Result<File, Error> {
        Self::options().read(true).openat(None, path)
    }

    /// Returns a new `OpenOptions` object.
    pub fn options() -> OpenOptions {
        OpenOptions::new()
    }

    /// Extracts the raw file descriptor.
    pub fn as_raw_fd(&self) -> c_int {
        self.rawfd.0
    }

    /// Reads from a file into a buffer.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        match unistd::read(self.rawfd.0, buf.as_mut_ptr(), buf.len() as u32) {
            -1 => {
                // Get `errno` and reset it.
                let errno: c_int = unsafe {
                    let errno: c_int = errno::errno;
                    errno::errno = 0;
                    errno
                };
                ::nvx::error!("read(): failed to read from file descriptor (errno={:?})", errno);
                Err(Error { errno })
            },
            ret => Ok(ret as usize),
        }
    }

    /// Moves the offset of a file descriptor.
    pub fn seek(&mut self, offset: FileOffset, whence: FileWhence) -> Result<off_t, Error> {
        match unistd::lseek(self.rawfd.0, offset.value(), whence as c_int) {
            -1 => {
                // Get `errno` and reset it.
                let errno: c_int = unsafe {
                    let errno: c_int = errno::errno;
                    errno::errno = 0;
                    errno
                };
                ::nvx::error!("seek(): failed to move file offset (errno={:?})", errno);
                Err(Error { errno })
            },
            newoffset => Ok(newoffset),
        }
    }

    /// Returns the current offset of a file descriptor.
    pub fn tell(&self) -> Result<off_t, Error> {
        match unistd::lseek(self.rawfd.0, 0, unistd::SEEK_CUR) {
            -1 => {
                // Get `errno` and reset it.
                let errno: c_int = unsafe {
                    let errno: c_int = errno::errno;
                    errno::errno = 0;
                    errno
                };
                ::nvx::error!("tell(): failed to get file offset (errno={:?})", errno);
                Err(Error { errno })
            },
            offset => Ok(offset),
        }
    }

    /// Writes a buffer to a file.
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        match unistd::write(self.rawfd.0, buf.as_ptr(), buf.len() as u32) {
            -1 => {
                // Get `errno` and reset it.
                let errno: c_int = unsafe {
                    let errno: c_int = errno::errno;
                    errno::errno = 0;
                    errno
                };
                ::nvx::error!("write(): failed to write to file descriptor (errno={:?})", errno);
                Err(Error { errno })
            },
            ret => Ok(ret as usize),
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // Do not close STDOUT, STDERR, or STDIN as it is shared with the runtime.
        if self.rawfd.0 == unistd::STDOUT_FILENO
            || self.rawfd.0 == unistd::STDERR_FILENO
            || self.rawfd.0 == unistd::STDIN_FILENO
        {
            return;
        }

        match unistd::close(self.rawfd.0) {
            // Success.
            0 => (),
            // Fail.
            -1 => {
                // Get `errno` and reset it.
                let errno: c_int = unsafe {
                    let errno: c_int = errno::errno;
                    errno::errno = 0;
                    errno
                };
                ::nvx::error!("failed to close file descriptor (errno={:?})", errno);
                // NOTE: We ignore errors on close, as the standard library does.
            },
            // Impossible.
            ret => unreachable!("close() returned an impossible value ({:?})", ret),
        }
    }
}

//==================================================================================================
// OpenOptions
//==================================================================================================

/// Options and flags which can be used to configure how a file is opened.
pub struct OpenOptions {
    append: bool,
    create: bool,
    create_new: bool,
    read: bool,
    truncate: bool,
    write: bool,
}

impl OpenOptions {
    /// Creates a blank new set of options ready for configuration.
    pub fn new() -> OpenOptions {
        OpenOptions {
            append: false,
            create: false,
            create_new: false,
            read: false,
            truncate: false,
            write: false,
        }
    }

    /// Sets the option for the append mode.
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.append = append;
        self
    }

    /// Sets the option for creating a file if it does not exist.
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.create = create;
        self
    }

    /// Sets the option for creating a new file, failing if it already exists.
    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.create_new = create_new;
        self
    }

    /// Opens a file at `path` with the options specified by `self`.
    pub fn openat(&self, dir: Option<&File>, path: &Path) -> Result<File, Error> {
        let mut flags: OpenFlags = OpenFlags::empty();
        flags.non_exclusive.append = self.append;
        flags.non_exclusive.create = self.create | self.create_new;
        flags.non_exclusive.exclusive = self.create_new;
        flags.non_exclusive.truncate = self.truncate;

        if self.read && self.write {
            flags.exclusive = ExclusiveOpenFlags::ReadWrite;
        } else if self.read {
            flags.exclusive = ExclusiveOpenFlags::ReadOnly;
        } else if self.write {
            flags.exclusive = ExclusiveOpenFlags::WriteOnly;
        } else {
            ::nvx::error!("openat(): invalid file mode");
            return Err(Error {
                errno: ErrorCode::InvalidArgument.into_errno(),
            });
        }

        let mode: mode_t = 0;
        let dirfd: c_int = match dir {
            Some(dir) => dir.as_raw_fd(),
            None => fcntl::AT_FDCWD,
        };
        match fcntl::openat(dirfd, &path.name, flags.into(), mode) {
            Ok(fd) => Ok(File { rawfd: Fd(fd) }),
            Err(error) => Err(Error {
                errno: error.code.into_errno(),
            }),
        }
    }

    /// Sets the option for read mode.
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.read = read;
        self
    }

    /// Sets the option for truncating a previous file.
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.truncate = truncate;
        self
    }

    /// Sets the option for write mode.
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.write = write;
        self
    }
}

//==================================================================================================

pub enum ExclusiveOpenFlags {
    ReadOnly,
    ReadWrite,
    WriteOnly,
}

#[derive(Default)]
pub struct NonExclusiveOpenFlags {
    pub append: bool,
    pub close_on_exec: bool,
    pub close_on_fork: bool,
    pub create: bool,
    pub directory: bool,
    pub dsync: bool,
    pub exclusive: bool,
    pub no_controlling_terminal: bool,
    pub no_follow: bool,
    pub non_block: bool,
    pub rsync: bool,
    pub sync: bool,
    pub truncate: bool,
    pub initialize_tty: bool,
}

pub struct OpenFlags {
    pub exclusive: ExclusiveOpenFlags,
    pub non_exclusive: NonExclusiveOpenFlags,
}

impl OpenFlags {
    pub fn empty() -> Self {
        Self {
            exclusive: ExclusiveOpenFlags::ReadOnly,
            non_exclusive: NonExclusiveOpenFlags::default(),
        }
    }
}

impl From<OpenFlags> for c_int {
    fn from(oflags: OpenFlags) -> c_int {
        let mut flags: c_int = 0;

        match oflags.exclusive {
            ExclusiveOpenFlags::ReadOnly => flags |= posix::fcntl::O_RDONLY,
            ExclusiveOpenFlags::ReadWrite => flags |= posix::fcntl::O_RDWR,
            ExclusiveOpenFlags::WriteOnly => flags |= posix::fcntl::O_WRONLY,
        }

        if oflags.non_exclusive.append {
            flags |= posix::fcntl::O_APPEND;
        }

        if oflags.non_exclusive.close_on_exec {
            unimplemented!("close-on-exec not supported yet");
        }

        if oflags.non_exclusive.close_on_fork {
            unimplemented!("close-on-fork not supported yet");
        }

        if oflags.non_exclusive.create {
            flags |= posix::fcntl::O_CREAT;
        }

        if oflags.non_exclusive.directory {
            flags |= posix::fcntl::O_DIRECTORY;
        }

        if oflags.non_exclusive.dsync {
            flags |= posix::fcntl::O_DSYNC;
        }

        if oflags.non_exclusive.exclusive {
            flags |= posix::fcntl::O_EXCL;
        }

        if oflags.non_exclusive.no_controlling_terminal {
            unimplemented!("no-controlling-terminal not supported yet");
        }

        if oflags.non_exclusive.no_follow {
            unimplemented!("no-follow not supported yet");
        }

        if oflags.non_exclusive.non_block {
            flags |= posix::fcntl::O_NONBLOCK;
        }

        if oflags.non_exclusive.rsync {
            flags |= posix::fcntl::O_RSYNC;
        }

        if oflags.non_exclusive.sync {
            flags |= posix::fcntl::O_SYNC;
        }

        if oflags.non_exclusive.truncate {
            flags |= posix::fcntl::O_TRUNC;
        }

        if oflags.non_exclusive.initialize_tty {
            unimplemented!("initialize-tty not supported yet");
        }

        flags
    }
}
