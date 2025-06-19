// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::size_t,
};

//==================================================================================================
// Constants
//==================================================================================================

pub mod file_seek {
    use crate::ffi::c_int;

    /// Seek relative to start-of-file.
    pub const SEEK_SET: c_int = 0;
    /// Seek relative to current position.
    pub const SEEK_CUR: c_int = 1;
    /// Seek relative to end-of-file.
    pub const SEEK_END: c_int = 2;
    /// Seek forwards from offset relative to start-of-file for a position within a hole.
    pub const SEEK_HOLE: c_int = 3;
    /// Seek forwards from offset relative to start-of-file for a position not within a hole.
    pub const SEEK_DATA: c_int = 4;
}

/// File number of standard input.
pub const STDIN_FILENO: i32 = 0;
/// File number of standard output.
pub const STDOUT_FILENO: i32 = 1;
/// File number of standard error.
pub const STDERR_FILENO: i32 = 2;

//==================================================================================================
// Function Prototypes
//==================================================================================================

unsafe extern "C" {
    pub unsafe fn getentropy(_buffer: *mut c_void, _length: size_t) -> c_int;
}
