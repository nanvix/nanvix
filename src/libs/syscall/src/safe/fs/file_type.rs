// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::sys::{
    stat::file_mode,
    types::mode_t,
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
}

impl From<mode_t> for FileType {
    fn from(mode: mode_t) -> Self {
        if file_mode::S_ISFIFO(mode) {
            FileType::Fifo
        } else if file_mode::S_ISCHR(mode) {
            FileType::CharacterDevice
        } else if file_mode::S_ISDIR(mode) {
            FileType::Directory
        } else if file_mode::S_ISBLK(mode) {
            FileType::BlockDevice
        } else if file_mode::S_ISREG(mode) {
            FileType::RegularFile
        } else if file_mode::S_ISLNK(mode) {
            FileType::SymbolicLink
        } else if file_mode::S_ISSOCK(mode) {
            FileType::Socket
        } else {
            FileType::Unknown
        }
    }
}
