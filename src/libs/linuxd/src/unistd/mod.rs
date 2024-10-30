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

/// Seek relative to start-of-file.
pub const SEEK_SET: i32 = 0;
/// Seek relative to current position.
pub const SEEK_CUR: i32 = 1;
/// Seek relative to end-of-file.
pub const SEEK_END: i32 = 2;
/// Seek forwards from offset relative to start-of-file for a position within a hole.
pub const SEEK_HOLE: i32 = 3;
/// Seek forwards from offset relative to start-of-file for a position not within a hole.
pub const SEEK_DATA: i32 = 4;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            close,
            fdatasync,
            fsync,
            lseek,
            ftruncate,
            write,
        };
    }
}
