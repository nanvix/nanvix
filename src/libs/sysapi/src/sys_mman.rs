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

pub mod msync_flags {
    use crate::ffi::c_int;

    /// Perform asynchronous writes.
    pub const MS_ASYNC: c_int = 0x1;
    /// Invalidate cached copies of mapped data.
    pub const MS_INVALIDATE: c_int = 0x2;
    /// Perform synchronous writes.
    pub const MS_SYNC: c_int = 0x4;
}

/// Advice values for use with `madvise()`. Values match Linux.
pub mod memory_advice {
    use crate::ffi::c_int;

    /// No special treatment.
    pub const MADV_NORMAL: c_int = 0;
    /// Expect page references in random order.
    pub const MADV_RANDOM: c_int = 1;
    /// Expect page references in sequential order.
    pub const MADV_SEQUENTIAL: c_int = 2;
    /// Expect access in the near future.
    pub const MADV_WILLNEED: c_int = 3;
    /// Do not expect access in the near future.
    pub const MADV_DONTNEED: c_int = 4;
    /// Contents of the pages may be discarded.
    pub const MADV_FREE: c_int = 8;
}

/// Flags for use with `mlockall()`. Values match Linux.
pub mod mlockall_flags {
    use crate::ffi::c_int;

    /// Lock all pages currently mapped into the address space.
    pub const MCL_CURRENT: c_int = 0x1;
    /// Lock all pages that become mapped into the address space in the future.
    pub const MCL_FUTURE: c_int = 0x2;
}
