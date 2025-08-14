// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

pub mod prot_flags {
    use crate::ffi::c_int;

    /// Page cannot be accessed.
    pub const PROT_NONE: c_int = 0;
    /// Page can be read.
    pub const PROT_READ: c_int = 1;
    /// Page can be written.
    pub const PROT_WRITE: c_int = 2;
    /// Page can be executed.
    pub const PROT_EXEC: c_int = 4;
}

pub mod flags {
    use crate::ffi::c_int;

    /// Map shared memory region.
    pub const MAP_SHARED: c_int = 0x01;
    /// Map private memory region.
    pub const MAP_PRIVATE: c_int = 0x02;
    /// Map memory region with fixed address.
    pub const MAP_FIXED: c_int = 0x10;
    /// Map memory region with anonymous allocation.
    pub const MAP_ANONYMOUS: c_int = 0x20;
}

/// Used to indicate a failure in `mmap()`.
pub const MAP_FAILED: *mut u8 = -1isize as *mut u8;
