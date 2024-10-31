// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::types::size_t;

//==================================================================================================

///
/// # Description
///
/// This structure represents an I/O vector.
///
pub struct iovec {
    /// Base address of a memory region for input or output.
    pub iov_base: *mut u8,
    /// The size of the memory pointer to by `iov_base`.
    pub iov_len: size_t,
}

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            writev,
            readv,
            pwritev,
        };
    }
}
