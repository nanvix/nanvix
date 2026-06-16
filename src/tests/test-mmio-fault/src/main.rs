// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate libc_string;
extern crate nvx;
extern crate nvx_crt0;

use ::arch::mem::PAGE_SIZE;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm,
        pm,
    },
    mm::Address,
    pm::Capability,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Encoded 8-byte "RAMFS   " tag exposed by the MicroVM RAMFS MMIO region.
const RAMFS_MMIO_TAG: u64 = u64::from_be_bytes(*b"RAMFS   ");

//==================================================================================================
// Entry Point
//==================================================================================================

///
/// # Description
///
/// Entry point of the MMIO fault test. This test validates that writing to an address just past
/// the end of a mapped MMIO region triggers a page fault.
///
/// The test allocates the RAMFS MMIO region, queries its base address and size, then writes one
/// byte past the end of the region. Because the page at `base + size` is not part of any mapping,
/// the write triggers a page fault.
///
/// # Expected Behavior
///
/// The write triggers a page fault. The memory daemon (`memd`) terminates the faulting process,
/// which exits with error code 3 (`ESRCH`).
///
#[no_mangle]
pub fn main() -> Result<(), Error> {
    // Acquire IO management capability.
    pm::__kcall_capctl(Capability::IoManagement, true)?;

    // Allocate the RAMFS MMIO region so its pages are mapped.
    mm::__kcall_mmio_alloc(RAMFS_MMIO_TAG)?;

    // Query the region's base address and size.
    let info: ::sys::mm::MmioRegionInfo = mm::__kcall_mmio_info(RAMFS_MMIO_TAG)?;
    let base_addr: usize = info.base().into_raw_value();
    let size: usize = info.size();

    // Align the end of the region up to the next page boundary so we are guaranteed to be past
    // every page the kernel mapped for this MMIO region.
    let unmapped_addr: usize = (base_addr + size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    ::syslog::info!(
        "test-mmio-fault: ramfs base={:#010x}, size={}, target={:#010x}",
        base_addr,
        size,
        unmapped_addr,
    );

    // SAFETY: This write intentionally triggers a page fault. The address is past the last
    // page-aligned boundary of the MMIO region and is guaranteed to be unmapped.
    unsafe {
        core::ptr::write_volatile(unmapped_addr as *mut u8, 0x42);
    }

    // Unreachable: the write above always triggers a page fault.
    Err(Error::new(ErrorCode::BadAddress, "expected page fault did not occur"))
}
