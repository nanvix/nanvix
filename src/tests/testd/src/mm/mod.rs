// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::{
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
    pm::{
        Capability,
        ProcessIdentifier,
    },
    sys::{
        arch::mem,
        config,
        config::{
            kernel,
            memory_layout,
        },
    },
};

//==================================================================================================
// Tests mmap() and munmap()
//==================================================================================================

///
/// # Description
///
/// Attempts to map as read only and then unmap it.
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

    let mypid: ProcessIdentifier = match nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = config::memory_layout::USER_HEAP_BASE;

    // Map a page.
    match nvx::mm::mmap(mypid, vaddr, AccessPermission::RDONLY) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Unmap the page.
    match nvx::mm::munmap(mypid, vaddr) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Release memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, false) {
        Ok(()) => (),
        _ => return false,
    }

    true
}

///
/// # Description
///
/// Attempts to map a page as writable, write to it, and then unmap it.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_mmap_write_munmap() -> bool {
    // Acquire memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = config::memory_layout::USER_HEAP_BASE;

    // Map a page.
    match nvx::mm::mmap(mypid, vaddr, AccessPermission::WRONLY) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Write to the page.
    let data: [u8; 4] = [0x12, 0x34, 0x56, 0x78];
    let ptr: *mut u8 = vaddr.into_raw_value() as *mut u8;
    unsafe {
        ptr.copy_from(data.as_ptr(), data.len());
    }

    // Check if contents are correct.
    let mut read_data: [u8; 4] = [0; 4];
    unsafe {
        ptr.copy_to(read_data.as_mut_ptr(), read_data.len());
    }
    if data != read_data {
        return false;
    }

    // Unmap the page.
    match nvx::mm::munmap(mypid, vaddr) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Release memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, false) {
        Ok(()) => (),
        _ => return false,
    }

    true
}

///
/// # Description
///
/// Attempts to map and unmap a page many times.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_mmap_munmap_many_times_inplace() -> bool {
    // Acquire memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let ntimes: usize = (kernel::MEMORY_SIZE / 8) / mem::PAGE_SIZE;

    for _ in 0..ntimes {
        let vaddr: VirtualAddress = memory_layout::USER_HEAP_BASE;

        // Map a page.
        match nvx::mm::mmap(mypid, vaddr, AccessPermission::RDONLY) {
            Ok(_) => (),
            Err(_) => return false,
        }

        // Unmap the page.
        match nvx::mm::munmap(mypid, vaddr) {
            Ok(_) => (),
            Err(_) => return false,
        }
    }

    // Release memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, false) {
        Ok(()) => (),
        _ => return false,
    }

    true
}

///
/// # Description
///
/// Attempts to map and unmap a page many times.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_mmap_munmap_many_times_rolling() -> bool {
    // Acquire memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let ntimes: usize = (kernel::MEMORY_SIZE / 8) / mem::PAGE_SIZE;

    for vaddr in (0..ntimes).map(|i| memory_layout::USER_HEAP_BASE_RAW + i * mem::PAGE_SIZE) {
        let vaddr: VirtualAddress = match VirtualAddress::from_raw_value(vaddr) {
            Ok(vaddr) => vaddr,
            Err(_) => return false,
        };

        // Map a page.
        match nvx::mm::mmap(mypid, vaddr, AccessPermission::RDONLY) {
            Ok(_) => (),
            Err(_) => return false,
        }

        // Unmap the page.
        match nvx::mm::munmap(mypid, vaddr) {
            Ok(_) => (),
            Err(_) => return false,
        }
    }

    // Release memory management capability.
    match nvx::pm::capctl(Capability::MemoryManagement, false) {
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
    crate::test!(test_mmap_write_munmap());
    crate::test!(test_mmap_munmap_many_times_inplace());
    crate::test!(test_mmap_munmap_many_times_rolling());
}
