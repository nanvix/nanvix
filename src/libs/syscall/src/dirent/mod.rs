// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::collections::VecDeque;
use ::sysapi::{
    dirent::{
        dirent,
        dirent_file_type::{
            DT_BLK,
            DT_CHR,
            DT_DIR,
            DT_FIFO,
            DT_LNK,
            DT_MQ,
            DT_REG,
            DT_SEM,
            DT_SHM,
            DT_SOCK,
            DT_TMO,
            DT_UNKNOWN,
        },
        posix_dent,
    },
    ffi::{
        c_int,
        c_uchar,
    },
};

//==================================================================================================
// Exports
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::posix_getdents;
        pub use self::syscall::opendir;
        pub use self::syscall::closedir;
        pub use self::syscall::readdir;

        pub mod bindings;
    }
}

//==================================================================================================
// File Types
//==================================================================================================

#[repr(u8)]
pub enum DirectoryEntryFileType {
    /// Unknown file type.
    Unknown = DT_UNKNOWN,
    /// FIFO special file.
    Fifo = DT_FIFO,
    /// Character special file.
    CharacterDevice = DT_CHR,
    /// Directory.
    Directory = DT_DIR,
    /// Block special file.
    BlockDevice = DT_BLK,
    /// Regular file.
    RegularFile = DT_REG,
    /// Symbolic link.
    SymbolicLink = DT_LNK,
    /// Socket.
    Socket = DT_SOCK,
    /// Message queue.
    MessageQueue = DT_MQ,
    /// Semaphore.
    Semaphore = DT_SEM,
    /// Shared memory object.
    SharedMemoryObject = DT_SHM,
    /// Typed memory object.
    TypedMemoryObject = DT_TMO,
}

impl From<c_uchar> for DirectoryEntryFileType {
    fn from(value: c_uchar) -> Self {
        match value {
            DT_FIFO => DirectoryEntryFileType::Fifo,
            DT_CHR => DirectoryEntryFileType::CharacterDevice,
            DT_DIR => DirectoryEntryFileType::Directory,
            DT_BLK => DirectoryEntryFileType::BlockDevice,
            DT_REG => DirectoryEntryFileType::RegularFile,
            DT_LNK => DirectoryEntryFileType::SymbolicLink,
            DT_SOCK => DirectoryEntryFileType::Socket,
            DT_MQ => DirectoryEntryFileType::MessageQueue,
            DT_SEM => DirectoryEntryFileType::Semaphore,
            DT_SHM => DirectoryEntryFileType::SharedMemoryObject,
            DT_TMO => DirectoryEntryFileType::TypedMemoryObject,
            _ => DirectoryEntryFileType::Unknown,
        }
    }
}

impl From<DirectoryEntryFileType> for c_uchar {
    fn from(file_type: DirectoryEntryFileType) -> Self {
        file_type as c_uchar
    }
}

//==================================================================================================
// Directory Stream Structure
//==================================================================================================

///
/// # Description
///
/// A type that represents a directory stream.
///
#[derive(Debug)]
pub struct DirectoryStream {
    /// File descriptor.
    fd: c_int,
    /// Next entries in the directory.
    next_entries: VecDeque<posix_dent>,
    /// Last directory entry returned by `readdir()`.
    last_entry: dirent,
}

impl DirectoryStream {
    /// Creates a new directory stream.
    pub fn new(fd: c_int) -> Self {
        Self {
            fd,
            next_entries: VecDeque::new(),
            last_entry: dirent::default(),
        }
    }

    /// Gets the file descriptor of the directory stream.
    pub fn fd(&self) -> c_int {
        self.fd
    }

    /// Pushes an entry into the directory stream.
    pub fn push(&mut self, posix_dent: posix_dent) {
        self.next_entries.push_back(posix_dent);
    }

    /// Pops the next entry from the directory stream.
    pub fn pop(&mut self) -> Option<posix_dent> {
        self.next_entries.pop_front()
    }

    /// Returns a reference to the last directory entry returned by `readdir()`.
    pub fn last_dirent_as_mut(&mut self) -> &mut dirent {
        &mut self.last_entry
    }
}

pub type DIR = DirectoryStream;
