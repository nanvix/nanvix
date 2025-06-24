// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys_types::c_size_t;

//==================================================================================================

/// An I/O vector.
pub struct iovec {
    /// Base address of a memory region for input or output.
    pub iov_base: *mut u8,
    /// The size of the memory pointer to by `iov_base`.
    pub iov_len: c_size_t,
}
