// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Exports
//==================================================================================================

pub use ::arch::mem::{
    PAGE_ALIGNMENT,
    PAGE_SIZE,
};

pub use ::sys::mm::*;

#[cfg(target_os = "none")]
pub use ::sys::kcall::mm::{
    mmap,
    munmap,
};
