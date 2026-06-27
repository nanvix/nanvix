// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::Error,
    mm::{
        Address,
        VirtualAddress,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_mman::prot_flags::{
        PROT_READ,
        PROT_WRITE,
    },
};
use ::syscall::sys::mman::{
    self,
    MemoryMapProtectionFlags,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Page size used for mmap tests.
const PAGE_SIZE: usize = 4096;

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Executes the mmap/munmap test.
pub fn run() -> Result<(), Error> {
    test_mmap_munmap()?;
    Ok(())
}

/// Tests whether we can map and unmap memory using `mmap()` and `munmap()`.
fn test_mmap_munmap() -> Result<(), Error> {
    let prot_flags: c_int = PROT_READ | PROT_WRITE;
    let prot: MemoryMapProtectionFlags = MemoryMapProtectionFlags::try_from(prot_flags)?;

    // Map a page of anonymous memory.
    let vaddr: VirtualAddress = mman::mmap(PAGE_SIZE, prot)?;

    // Write to the mapped memory.
    let ptr: *mut u8 = core::ptr::without_provenance_mut(vaddr.into_raw_value());
    unsafe { *ptr = b'A' };

    // Verify the write.
    assert_eq!(unsafe { *ptr }, b'A');

    // Unmap the page.
    mman::munmap(vaddr, PAGE_SIZE)?;

    Ok(())
}
