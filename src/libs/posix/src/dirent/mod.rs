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

use crate::{
    ffi::{
        c_char,
        c_int,
        c_uchar,
    },
    limits::NAME_MAX,
    sys::types::{
        ino_t,
        reclen_t,
        size_t,
    },
};

//==================================================================================================
// Exports
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::posix_getdents;
    }
}

#[cfg(all(feature = "syscall", feature = "staticlib"))]
pub mod bindings;

//==================================================================================================
// File Types
//==================================================================================================

///
/// # Description
///
/// File types for `d_type` field in `posix_dent` structure.
///
mod file_type {
    pub use super::*;

    /// Unknown file type.
    pub const DT_UNKNOWN: c_uchar = 0;
    /// FIFO special file.
    pub const DT_FIFO: c_uchar = 1;
    /// Character special file.
    pub const DT_CHR: c_uchar = 2;
    /// Directory.
    pub const DT_DIR: c_uchar = 4;
    /// Block special file.
    pub const DT_BLK: c_uchar = 6;
    /// Regular file.
    pub const DT_REG: c_uchar = 8;
    /// Symbolic link.
    pub const DT_LNK: c_uchar = 10;
    /// Socket.
    pub const DT_SOCK: c_uchar = 12;
    /// Message queue.
    pub const DT_MQ: c_uchar = 13;
    /// Semaphore.
    pub const DT_SEM: c_uchar = 14;
    /// Shared memory object.
    pub const DT_SHM: c_uchar = 15;
}
use alloc::{
    fmt,
    string::{
        String,
        ToString,
    },
};
pub use file_type::*;

//==================================================================================================
// Directory Stream Structure
//==================================================================================================

///
/// # Description
///
/// A type that represents a directory stream.
///
#[repr(C, packed)]
pub struct DIR {
    /// File descriptor.
    fd: c_int,
    /// Flags.
    flags: c_int,
    /// Number of valid entries left in the buffer.
    valid: size_t,
    /// Next valid entry in the buffer.
    next: *mut dirent,
    /// Buffer of directory entries.
    buffer: *mut dirent,
}
::nvx::sys::static_assert_size!(DIR, DIR::_SIZE_OF_DIR);

impl DIR {
    /// Size of `fd` field, used for static assertions.
    const _SIZE_OF_FD: usize = core::mem::size_of::<c_int>();
    /// Size of `flags` field, used for static assertions.
    const _SIZE_OF_FLAGS: usize = core::mem::size_of::<c_int>();
    /// Size of `valid` field, used for static assertions.
    const _SIZE_OF_VALID: usize = core::mem::size_of::<size_t>();
    /// Size of `next` field, used for static assertions.
    const _SIZE_OF_NEXT: usize = core::mem::size_of::<*mut dirent>();
    /// Size of `buffer` field, used for static assertions.
    const _SIZE_OF_BUFFER: usize = core::mem::size_of::<*mut dirent>();
    /// Size of `DIR` struct, used for static assertions.
    const _SIZE_OF_DIR: usize = Self::_SIZE_OF_FD
        + Self::_SIZE_OF_FLAGS
        + Self::_SIZE_OF_VALID
        + Self::_SIZE_OF_NEXT
        + Self::_SIZE_OF_BUFFER;
}

//==================================================================================================
// Directory Entry Structure
//==================================================================================================

///
/// # Description
///
/// A type representing a directory entry.
///
#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct dirent {
    /// File serial number.
    pub d_ino: ino_t,
    /// File name (including null terminator character).
    pub d_name: [c_char; NAME_MAX + 1],
}
::nvx::sys::static_assert_size!(dirent, dirent::_SIZE_OF_DIRENT);

impl dirent {
    /// Size of `d_ino` field, used for static assertions.
    const _SIZE_OF_D_INO: usize = core::mem::size_of::<ino_t>();
    /// Size of `d_name` field, used for static assertions.
    const _SIZE_OF_D_NAME: usize = core::mem::size_of::<[c_char; NAME_MAX + 1]>();
    /// Size of `dirent` struct, used for static assertions.
    const _SIZE_OF_DIRENT: usize = Self::_SIZE_OF_D_INO + Self::_SIZE_OF_D_NAME;
}

//==================================================================================================
// Posix Directory Entry Structure
//==================================================================================================

///
/// # Description
///
/// A type representing a POSIX directory entry.
///
#[repr(C, packed)]
pub struct posix_dent {
    /// File serial number.
    pub d_ino: ino_t,
    /// Length of this entry, including trailing padding if necessary.
    pub d_reclen: reclen_t,
    /// File type.
    pub d_type: c_uchar,
    /// File name (including null terminator character).
    pub d_name: [c_char; NAME_MAX + 1],
}
::nvx::sys::static_assert_size!(posix_dent, posix_dent::_SIZE_OF_POSIX_DIRENT);

impl posix_dent {
    /// Size of `d_ino` field, used for static assertions.
    const _SIZE_OF_D_INO: usize = core::mem::size_of::<ino_t>();
    /// Size of `d_reclen` field, used for static assertions.
    const _SIZE_OF_D_RECLEN: usize = core::mem::size_of::<reclen_t>();
    /// Size of `d_type` field, used for static assertions.
    const _SIZE_OF_D_TYPE: usize = core::mem::size_of::<c_uchar>();
    /// Size of `d_name` field, used for static assertions.
    const _SIZE_OF_D_NAME: usize = core::mem::size_of::<[c_char; NAME_MAX + 1]>();
    /// Size of `posix_dirent` struct, used for static assertions.
    const _SIZE_OF_POSIX_DIRENT: usize = Self::_SIZE_OF_D_INO
        + Self::_SIZE_OF_D_RECLEN
        + Self::_SIZE_OF_D_TYPE
        + Self::_SIZE_OF_D_NAME;
}

impl Default for posix_dent {
    fn default() -> Self {
        Self {
            d_ino: 0,
            d_reclen: 0,
            d_type: 0,
            d_name: [0; NAME_MAX + 1],
        }
    }
}

impl fmt::Debug for posix_dent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let d_name = self
            .d_name
            .iter()
            .map(|&c| c as u8 as char)
            .collect::<String>()
            .trim_end_matches('\0')
            .to_string();
        write!(
            f,
            "posix_dent {{ d_ino: {}, d_reclen: {}, d_type: {}, d_name: {:?} }}",
            { self.d_ino },
            { self.d_reclen },
            self.d_type,
            d_name
        )
    }
}
