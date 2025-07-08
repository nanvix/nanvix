// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Constants
//==================================================================================================

use crate::{
    ffi::{
        c_char,
        c_uchar,
    },
    limits::NAME_MAX,
    sys_types::{
        ino_t,
        reclen_t,
    },
};
use ::core::fmt;

///
/// # Description
///
/// File types for `d_type` field in `posix_dent` structure.
///
pub mod dirent_file_type {
    use crate::ffi::c_uchar;

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
    /// Typed memory object.
    pub const DT_TMO: c_uchar = 16; // FIXME: https://github.com/nanvix/nanvix/issues/568
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// Directory entry.
///
#[repr(C, packed)]
pub struct dirent {
    /// File serial number.
    pub d_ino: ino_t,
    /// File name (including null terminator character).
    pub d_name: [c_uchar; NAME_MAX + 1],
}
::static_assert::assert_eq_size!(dirent, dirent::_SIZE_OF_DIRENT);

impl dirent {
    /// Size of `d_ino` field, used for static assertions.
    const _SIZE_OF_D_INO: usize = core::mem::size_of::<ino_t>();
    /// Size of `d_name` field, used for static assertions.
    const _SIZE_OF_D_NAME: usize = core::mem::size_of::<[c_uchar; NAME_MAX + 1]>();
    /// Size of `dirent` struct, used for static assertions.
    const _SIZE_OF_DIRENT: usize = Self::_SIZE_OF_D_INO + Self::_SIZE_OF_D_NAME;
}

impl From<posix_dent> for dirent {
    fn from(posix_dent: posix_dent) -> Self {
        Self {
            d_ino: posix_dent.d_ino,
            d_name: posix_dent.d_name,
        }
    }
}

impl fmt::Debug for dirent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dirent {{ d_ino: {} }}", { self.d_ino })
    }
}

impl Default for dirent {
    fn default() -> Self {
        Self {
            d_ino: 0,
            d_name: [0; NAME_MAX + 1],
        }
    }
}

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
    pub d_name: [c_uchar; NAME_MAX + 1],
    /// Padding.
    pub _padding: [c_char; 1],
}
::static_assert::assert_eq_size!(posix_dent, posix_dent::_SIZE_OF_POSIX_DIRENT);

impl posix_dent {
    /// Size of `d_ino` field, used for static assertions.
    const _SIZE_OF_D_INO: usize = core::mem::size_of::<ino_t>();
    /// Size of `d_reclen` field, used for static assertions.
    const _SIZE_OF_D_RECLEN: usize = core::mem::size_of::<reclen_t>();
    /// Size of `d_type` field, used for static assertions.
    const _SIZE_OF_D_TYPE: usize = core::mem::size_of::<c_uchar>();
    /// Size of `d_name` field, used for static assertions.
    const _SIZE_OF_D_NAME: usize = core::mem::size_of::<[c_uchar; NAME_MAX + 1]>();
    /// Size of `_padding` field, used for static assertions.
    const _SIZE_OF_PADDING: usize = core::mem::size_of::<[c_char; 1]>();
    /// Size of `posix_dirent` struct, used for static assertions.
    const _SIZE_OF_POSIX_DIRENT: usize = Self::_SIZE_OF_D_INO
        + Self::_SIZE_OF_D_RECLEN
        + Self::_SIZE_OF_D_TYPE
        + Self::_SIZE_OF_D_NAME
        + Self::_SIZE_OF_PADDING;
}

impl Default for posix_dent {
    fn default() -> Self {
        Self {
            d_ino: 0,
            d_reclen: 0,
            d_type: 0,
            d_name: [0; NAME_MAX + 1],
            _padding: [0],
        }
    }
}

impl fmt::Debug for posix_dent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "posix_dent {{ d_ino: {}, d_reclen: {}, d_type: {} }}",
            { self.d_ino },
            { self.d_reclen },
            self.d_type,
        )
    }
}
