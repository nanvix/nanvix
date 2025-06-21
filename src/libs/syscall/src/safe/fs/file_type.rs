// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::dirent::DirectoryEntryFileType;
use ::sysapi::{
    sys_stat::file_type::{
        S_ISBLK,
        S_ISCHR,
        S_ISDIR,
        S_ISFIFO,
        S_ISLNK,
        S_ISREG,
        S_ISSOCK,
        S_TYPEISMQ,
        S_TYPEISSEM,
        S_TYPEISSHM,
        S_TYPEISTMO,
    },
    sys_types::mode_t,
};

//==================================================================================================
// File Type
//==================================================================================================

///
/// # Description
///
/// An enumeration that represents the type of a file in the file system.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    /// Unknown file type.
    Unknown,
    /// Named pipe.
    Fifo,
    /// Character device.
    CharacterDevice,
    /// Directory.
    Directory,
    /// Block device.
    BlockDevice,
    /// Regular file.
    RegularFile,
    /// Symbolic link.
    SymbolicLink,
    /// Socket.
    Socket,
    /// Message queue.
    MessageQueue,
    /// Semaphore.
    Semaphore,
    /// Shared memory object.
    SharedMemoryObject,
    /// Typed memory object.
    TypedMemoryObject,
}

impl From<mode_t> for FileType {
    fn from(mode: mode_t) -> Self {
        if S_ISFIFO(mode) {
            FileType::Fifo
        } else if S_ISCHR(mode) {
            FileType::CharacterDevice
        } else if S_ISDIR(mode) {
            FileType::Directory
        } else if S_ISBLK(mode) {
            FileType::BlockDevice
        } else if S_ISREG(mode) {
            FileType::RegularFile
        } else if S_ISLNK(mode) {
            FileType::SymbolicLink
        } else if S_ISSOCK(mode) {
            FileType::Socket
        } else if S_TYPEISMQ(mode) {
            FileType::MessageQueue
        } else if S_TYPEISSEM(mode) {
            FileType::Semaphore
        } else if S_TYPEISSHM(mode) {
            FileType::SharedMemoryObject
        } else if S_TYPEISTMO(mode) {
            FileType::TypedMemoryObject
        } else {
            FileType::Unknown
        }
    }
}

impl From<DirectoryEntryFileType> for FileType {
    fn from(entry_type: DirectoryEntryFileType) -> Self {
        match entry_type {
            DirectoryEntryFileType::Fifo => FileType::Fifo,
            DirectoryEntryFileType::CharacterDevice => FileType::CharacterDevice,
            DirectoryEntryFileType::Directory => FileType::Directory,
            DirectoryEntryFileType::BlockDevice => FileType::BlockDevice,
            DirectoryEntryFileType::RegularFile => FileType::RegularFile,
            DirectoryEntryFileType::SymbolicLink => FileType::SymbolicLink,
            DirectoryEntryFileType::Socket => FileType::Socket,
            DirectoryEntryFileType::MessageQueue => FileType::MessageQueue,
            DirectoryEntryFileType::Semaphore => FileType::Semaphore,
            DirectoryEntryFileType::SharedMemoryObject => FileType::SharedMemoryObject,
            DirectoryEntryFileType::TypedMemoryObject => FileType::TypedMemoryObject,
            _ => FileType::Unknown,
        }
    }
}
