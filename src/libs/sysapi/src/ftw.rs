// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// The entry is a regular file.
pub const FTW_F: c_int = 0;
/// The entry is a directory.
pub const FTW_D: c_int = 1;
/// The entry is a directory that cannot be read.
pub const FTW_DNR: c_int = 2;
/// The entry could not be inspected with `stat()`.
pub const FTW_NS: c_int = 3;
/// The entry is a symbolic link.
pub const FTW_SL: c_int = 4;
