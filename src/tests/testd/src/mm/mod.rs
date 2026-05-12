// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::sys::{
    config::memory_layout::USER_MMAP_END,
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

/// Number of pages to exercise in the "many times" tests.
const MM_TEST_NTIMES: usize = 64;

// NOTE: these low-level kcall tests map pages starting at `USER_MMAP_END`, which lies in the
// guard region between the unified mmap region and the user stack. This address is deliberately
// outside the bump-allocated mmap region so that direct kcall mappings do not conflict with
// heap or higher-level mmap reservations.

// Compile-time check: ensure the test pages fit within the guard region.
const _: () = assert!(
    ::config::memory_layout::USER_MMAP_END_RAW + MM_TEST_NTIMES * PAGE_SIZE
        <= ::config::memory_layout::USER_STACK_TOP_RAW,
    "testd mm tests: test pages overflow the guard region into the user stack",
);

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
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = USER_MMAP_END;

    // Map a page.
    match ::sys::kcall::mm::__kcall_mmap(mypid, vaddr, 1, AccessPermission::RDONLY) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Unmap the page.
    match ::sys::kcall::mm::__kcall_munmap(mypid, vaddr) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Release memory management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = USER_MMAP_END;

    // Map a page.
    match ::sys::kcall::mm::__kcall_mmap(mypid, vaddr, 1, AccessPermission::WRONLY) {
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
    match ::sys::kcall::mm::__kcall_munmap(mypid, vaddr) {
        Ok(_) => (),
        Err(_) => return false,
    }

    // Release memory management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    for _ in 0..MM_TEST_NTIMES {
        let vaddr: VirtualAddress = USER_MMAP_END;

        // Map a page.
        match ::sys::kcall::mm::__kcall_mmap(mypid, vaddr, 1, AccessPermission::RDONLY) {
            Ok(_) => (),
            Err(_) => return false,
        }

        // Unmap the page.
        match ::sys::kcall::mm::__kcall_munmap(mypid, vaddr) {
            Ok(_) => (),
            Err(_) => return false,
        }
    }

    // Release memory management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
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
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true) {
        Ok(()) => (),
        _ => return false,
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let mmap_end_raw: usize = USER_MMAP_END.into_raw_value();

    // Map pages at consecutive addresses starting at the guard region.
    for vaddr in (0..MM_TEST_NTIMES).map(|i| mmap_end_raw + i * PAGE_SIZE) {
        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);

        // Map a page.
        match ::sys::kcall::mm::__kcall_mmap(mypid, vaddr, 1, AccessPermission::RDONLY) {
            Ok(_) => (),
            Err(_) => return false,
        }

        // Unmap the page.
        match ::sys::kcall::mm::__kcall_munmap(mypid, vaddr) {
            Ok(_) => (),
            Err(_) => return false,
        }
    }

    // Release memory management capability.
    match ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false) {
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
    if ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true).is_err() {
        return false;
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let vaddr: VirtualAddress = USER_MMAP_END;

    // Map a page.
    if ::sys::kcall::mm::__kcall_mmap(mypid, vaddr, 1, AccessPermission::WRONLY).is_err() {
        return false;
    }

    // Fill page with ones.
    let data: [u8; PAGE_SIZE] = [0xFF; PAGE_SIZE];
    let ptr: *mut u8 = vaddr.into_raw_value() as *mut u8;
    unsafe {
        ptr.copy_from(data.as_ptr(), data.len());
    }

    // Unmap the page.
    if ::sys::kcall::mm::__kcall_munmap(mypid, vaddr).is_err() {
        return false;
    }

    // Map the page again to read.
    if ::sys::kcall::mm::__kcall_mmap(mypid, vaddr, 1, AccessPermission::RDONLY).is_err() {
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
    if ::sys::kcall::mm::__kcall_munmap(mypid, vaddr).is_err() {
        return false;
    }

    // Release memory management capability.
    if ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false).is_err() {
        return false;
    }

    true
}

//==================================================================================================
// Tests multi-page mmap()
//==================================================================================================

///
/// # Description
///
/// Maps multiple pages in a single `mmap()` call, verifies they are zeroed, and unmaps them.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_mmap_multi_page() -> bool {
    const NPAGES: usize = 4;

    // Acquire memory management capability.
    if ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, true).is_err() {
        return false;
    }

    let mypid: ProcessIdentifier = match ::sys::kcall::pm::__kcall_getpid() {
        Ok(pid) => pid,
        Err(_) => return false,
    };

    let base_vaddr: VirtualAddress = USER_MMAP_END;

    // Map multiple pages in a single call.
    if ::sys::kcall::mm::__kcall_mmap(mypid, base_vaddr, NPAGES, AccessPermission::RDWR).is_err() {
        return false;
    }

    // Verify all pages are zeroed.
    let zeros: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
    for i in 0..NPAGES {
        let page_addr: usize = base_vaddr.into_raw_value() + i * PAGE_SIZE;
        let mut data: [u8; PAGE_SIZE] = [0xFF; PAGE_SIZE];
        let ptr: *const u8 = page_addr as *const u8;
        unsafe {
            ptr.copy_to(data.as_mut_ptr(), data.len());
        }
        if zeros != data {
            return false;
        }
    }

    // Write a marker to each page.
    for i in 0..NPAGES {
        let page_addr: usize = base_vaddr.into_raw_value() + i * PAGE_SIZE;
        let ptr: *mut u8 = page_addr as *mut u8;
        unsafe {
            *ptr = (i + 1) as u8;
        }
    }

    // Read back markers.
    for i in 0..NPAGES {
        let page_addr: usize = base_vaddr.into_raw_value() + i * PAGE_SIZE;
        let ptr: *const u8 = page_addr as *const u8;
        let val: u8 = unsafe { *ptr };
        if val != (i + 1) as u8 {
            return false;
        }
    }

    // Unmap all pages.
    for i in 0..NPAGES {
        let page_addr: usize = base_vaddr.into_raw_value() + i * PAGE_SIZE;
        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(page_addr);
        if ::sys::kcall::mm::__kcall_munmap(mypid, vaddr).is_err() {
            return false;
        }
    }

    // Release memory management capability.
    if ::sys::kcall::pm::__kcall_capctl(Capability::MemoryManagement, false).is_err() {
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
    crate::test!(test_mmap_multi_page());
}
