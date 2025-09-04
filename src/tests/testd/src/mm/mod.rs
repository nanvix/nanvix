// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::config::memory_layout::{
    USER_MMAPPED_END_RAW,
    USER_MMAP_BASE_RAW,
};
use ::sys::{
    config::memory_layout::USER_MMAP_BASE,
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
    pm::{
        Capability,
        ProcessIdentifier,
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
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = USER_MMAP_BASE;

    // Map a page.
    match ::sys::kcall::mm::mmap(mypid, vaddr, AccessPermission::RDONLY) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Unmap the page.
    match ::sys::kcall::mm::munmap(mypid, vaddr) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Release memory management capability.
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, false) {
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
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = USER_MMAP_BASE;

    // Map a page.
    match ::sys::kcall::mm::mmap(mypid, vaddr, AccessPermission::WRONLY) {
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
    match ::sys::kcall::mm::munmap(mypid, vaddr) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Release memory management capability.
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, false) {
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
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let ntimes: usize = ((USER_MMAPPED_END_RAW - USER_MMAP_BASE_RAW) / 64) / PAGE_SIZE;

    for _ in 0..ntimes {
        let vaddr: VirtualAddress = USER_MMAP_BASE;

        // Map a page.
        match ::sys::kcall::mm::mmap(mypid, vaddr, AccessPermission::RDONLY) {
            Ok(_) => (),
            Err(_) => return false,
        }

        // Unmap the page.
        match ::sys::kcall::mm::munmap(mypid, vaddr) {
            Ok(_) => (),
            Err(_) => return false,
        }
    }

    // Release memory management capability.
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, false) {
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
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let ntimes: usize = ((USER_MMAPPED_END_RAW - USER_MMAP_BASE_RAW) / 64) / PAGE_SIZE;

    for vaddr in (0..ntimes).map(|i| USER_MMAP_BASE_RAW + i * PAGE_SIZE) {
        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);

        // Map a page.
        match ::sys::kcall::mm::mmap(mypid, vaddr, AccessPermission::RDONLY) {
            Ok(_) => (),
            Err(_) => return false,
        }

        // Unmap the page.
        match ::sys::kcall::mm::munmap(mypid, vaddr) {
            Ok(_) => (),
            Err(_) => return false,
        }
    }

    // Release memory management capability.
    match ::sys::kcall::pm::capctl(Capability::MemoryManagement, false) {
        Ok(()) => (),
        _ => return false,
    }

    true
}

///
/// # Description
///
/// Tests if `mmap()` always returns pages filled with zeros.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_mmap_munmap_return_zeros() -> bool {
    // Acquire memory management capability.
    if ::sys::kcall::pm::capctl(Capability::MemoryManagement, true).is_err() {
        return false;
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = USER_MMAP_BASE;

    // Map a page.
    if ::sys::kcall::mm::mmap(mypid, vaddr, AccessPermission::WRONLY).is_err() {
        return false;
    }

    // Fill page with ones.
    let data: [u8; PAGE_SIZE] = [0xFF; PAGE_SIZE];
    let ptr: *mut u8 = vaddr.into_raw_value() as *mut u8;
    unsafe {
        ptr.copy_from(data.as_ptr(), data.len());
    }

    // Unmap the page.
    if ::sys::kcall::mm::munmap(mypid, vaddr).is_err() {
        return false;
    }

    // Map the page again to read.
    if ::sys::kcall::mm::mmap(mypid, vaddr, AccessPermission::RDONLY).is_err() {
        return false;
    }

    // Check if page is filled with zeros.
    let zeros: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    let mut data: [u8; PAGE_SIZE] = [0xFF; PAGE_SIZE];
    unsafe {
        ptr.copy_to(data.as_mut_ptr(), data.len());
    }
    if zeros != data {
        return false;
    }

    // Unmap the page.
    if ::sys::kcall::mm::munmap(mypid, vaddr).is_err() {
        return false;
    }

    // Release memory management capability.
    if ::sys::kcall::pm::capctl(Capability::MemoryManagement, false).is_err() {
        return false;
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
    crate::test!(test_mmap_munmap_return_zeros());
}
