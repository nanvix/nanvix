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
    sys_stat::file_type,
    sys_types::{
        ino_t,
        mode_t,
        reclen_t,
    },
};
use ::core::fmt;

///
/// # Description
///
/// File types for `d_type` field in `posix_dent` structure.
///
/// Each file type, except for `DT_UNKNOWN`, is the file-type code of the matching `S_IF*` file
/// mode, that is, the value of the `S_IFMT` bits of that file mode.
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
    pub const DT_MQ: c_uchar = 3;
    /// Semaphore.
    pub const DT_SEM: c_uchar = 5;
    /// Shared memory object.
    pub const DT_SHM: c_uchar = 7;
    /// Typed memory object.
    pub const DT_TMO: c_uchar = 9;
}

/// Returns the file-type code of a file mode, that is, the value of its `S_IFMT` bits.
const fn file_type_code(mode: mode_t) -> c_uchar {
    ((mode & file_type::S_IFMT) >> file_type::S_IFMT.trailing_zeros()) as c_uchar
}

// Each file type is the file-type code of the matching file mode.
::static_assert::assert_eq!(dirent_file_type::DT_FIFO == file_type_code(file_type::S_IFIFO));
::static_assert::assert_eq!(dirent_file_type::DT_CHR == file_type_code(file_type::S_IFCHR));
::static_assert::assert_eq!(dirent_file_type::DT_DIR == file_type_code(file_type::S_IFDIR));
::static_assert::assert_eq!(dirent_file_type::DT_BLK == file_type_code(file_type::S_IFBLK));
::static_assert::assert_eq!(dirent_file_type::DT_REG == file_type_code(file_type::S_IFREG));
::static_assert::assert_eq!(dirent_file_type::DT_LNK == file_type_code(file_type::S_IFLNK));
::static_assert::assert_eq!(dirent_file_type::DT_SOCK == file_type_code(file_type::S_IFSOCK));
::static_assert::assert_eq!(dirent_file_type::DT_MQ == file_type_code(file_type::S_IFMQ));
::static_assert::assert_eq!(dirent_file_type::DT_SEM == file_type_code(file_type::S_IFSEM));
::static_assert::assert_eq!(dirent_file_type::DT_SHM == file_type_code(file_type::S_IFSHM));
::static_assert::assert_eq!(dirent_file_type::DT_TMO == file_type_code(file_type::S_IFTMO));

//==================================================================================================
// Structures
//==================================================================================================

///
/// Directory entry.
///
#[repr(C)]
pub struct dirent {
    /// File serial number.
    pub d_ino: ino_t,
    /// File name (including null terminator character).
    pub d_name: [c_uchar; NAME_MAX + 1],
}

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
#[repr(C)]
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
// Serialized size may differ from sizeof on 64-bit targets due to trailing alignment padding.

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
