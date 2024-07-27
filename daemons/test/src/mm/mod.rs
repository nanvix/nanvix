// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Tests mmap()
//==================================================================================================

use nvx::{
    AccessPermission,
    ProcessIdentifier,
};

////
/// # Description
///
/// Attempts to map and unmap a memory page.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_mmap_munmap() -> bool {
    // Acquire memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match nvx::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: usize = 0xa0000000;

    // Map a page.
    match nvx::mmap(mypid, vaddr, AccessPermission::RDONLY) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Unmap the page.
    match nvx::munmap(mypid, vaddr) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Release memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    true
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests kernel calls in the process management kernel calls.
///
pub fn test() {
    crate::test!(test_mmap_munmap());
}
