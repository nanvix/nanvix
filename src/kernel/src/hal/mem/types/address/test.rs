// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    Address,
    PageAligned,
    PageRange,
    PhysicalAddress,
    VirtualAddress,
};
use ::arch::mem::{
    PAGE_ALIGNMENT,
    PAGE_SIZE,
    PGTAB_ALIGNMENT,
};
use ::sys::{
    error::ErrorCode,
    mm::Alignment,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests if [`Address::from_raw_value()`] successfully instantiates address zero.
fn test_new_zero_address<T: Address>() -> bool {
    let raw_addr: usize = 0;
    match T::from_raw_value(raw_addr) {
        // The address was successfully created.
        Ok(_) => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_new_zero_address() -> bool {
    test_new_zero_address::<VirtualAddress>()
}

fn test_physical_address_new_zero_address() -> bool {
    test_new_zero_address::<PhysicalAddress>()
}

fn test_page_aligned_virtual_address_new_zero_address() -> bool {
    test_new_zero_address::<PageAligned<VirtualAddress>>()
}

fn test_page_aligned_physical_address_new_zero_address() -> bool {
    test_new_zero_address::<PageAligned<PhysicalAddress>>()
}

/// Tests if [`Address::from_raw_value()`] successfully instantiates the maximum address.
fn test_new_max_address<T: Address>() -> bool {
    let raw_addr: usize = T::max_addr();
    match T::from_raw_value(raw_addr) {
        // The address was successfully created.
        Ok(_) => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_new_max_address() -> bool {
    test_new_max_address::<VirtualAddress>()
}

fn test_physical_address_new_max_address() -> bool {
    test_new_max_address::<PhysicalAddress>()
}

/// Tests if [`Address::from_raw_value()`] successfully instantiates the maximum page-aligned address.
///
/// Unlike `test_new_max_address`, this function cannot delegate to the generic helper because
/// `PageAligned::max_addr()` returns the inner type's maximum, which is typically not page-aligned.
/// Instead, it computes the largest page-aligned address that fits within the address space.
fn test_page_aligned_new_max_address<T: Address>() -> bool {
    let raw_addr: usize = T::max_addr() & !(PAGE_SIZE - 1);
    match PageAligned::<T>::from_raw_value(raw_addr) {
        // The address was successfully created.
        Ok(_) => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create page-aligned max address (err={:?})", err);
            false
        },
    }
}

fn test_page_aligned_virtual_address_new_max_address() -> bool {
    test_page_aligned_new_max_address::<VirtualAddress>()
}

fn test_page_aligned_physical_address_new_max_address() -> bool {
    test_page_aligned_new_max_address::<PhysicalAddress>()
}

/// Tests if [`Address::from_raw_value()`] fails to create an out of range address.
fn test_new_out_of_range_address<T: Address>() -> bool {
    let raw_addr: usize = T::max_addr() + 1;
    match T::from_raw_value(raw_addr) {
        // The address was created and it should not.
        Ok(addr) => {
            error!("succeeded to create an invalid address (addr={:?})", addr);
            false
        },
        // The address was not created as expected.
        Err(e) if e.code == ErrorCode::BadAddress => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_physical_address_new_out_of_range_address() -> bool {
    test_new_out_of_range_address::<PhysicalAddress>()
}

/// Tests if [`PageAligned::from_raw_value()`] fails to create a page-aligned address that is out of range.
///
/// Uses the first page-aligned address beyond `T::max_addr()`. This is only applicable to address
/// types whose maximum is below `usize::MAX` (e.g., `PhysicalAddress`).
fn test_page_aligned_new_out_of_range_address<T: Address>() -> bool {
    let raw_addr: usize = (T::max_addr() & !(PAGE_SIZE - 1)) + PAGE_SIZE;
    match PageAligned::<T>::from_raw_value(raw_addr) {
        // The address was created and it should not.
        Ok(addr) => {
            error!("succeeded to create an out-of-range page-aligned address (addr={:?})", addr);
            false
        },
        // The address was not created as expected.
        Err(e) if e.code == ErrorCode::BadAddress => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_page_aligned_physical_address_new_out_of_range_address() -> bool {
    test_page_aligned_new_out_of_range_address::<PhysicalAddress>()
}

/// Tests if [`PageAligned::from_raw_value()`] fails to create an unaligned virtual address.
fn test_page_aligned_virtual_address_new_unaligned() -> bool {
    let raw_addr: usize = 1;
    match PageAligned::<VirtualAddress>::from_raw_value(raw_addr) {
        // The address was created and it should not.
        Ok(_) => {
            error!("succeeded to create an unaligned address (addr={})", raw_addr);
            false
        },
        // The address was not created as expected.
        Err(e) if e.code == ErrorCode::BadAddress => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

/// Tests if [`PageAligned::from_raw_value()`] fails to create an unaligned physical address.
fn test_page_aligned_physical_address_new_unaligned() -> bool {
    let raw_addr: usize = 1;
    match PageAligned::<PhysicalAddress>::from_raw_value(raw_addr) {
        // The address was created and it should not.
        Ok(_) => {
            error!("succeeded to create an unaligned address (addr={})", raw_addr);
            false
        },
        // The address was not created as expected.
        Err(e) if e.code == ErrorCode::BadAddress => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

/// Tests if [`Address::is_aligned()`] asserts an aligned address as aligned.
fn test_is_aligned_aligned<T: Address>() -> bool {
    let raw_addr: usize = PAGE_SIZE;
    let aligned_address: T = match T::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match aligned_address.is_aligned(PAGE_ALIGNMENT) {
        // An aligned address was asserted as aligned.
        Ok(true) => true,
        // An aligned address was not asserted as aligned.
        Ok(false) => {
            error!("aligned address was asserted as not aligned (addr={:?})", aligned_address);
            false
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_is_aligned_aligned() -> bool {
    test_is_aligned_aligned::<VirtualAddress>()
}

fn test_physical_address_is_aligned_aligned() -> bool {
    test_is_aligned_aligned::<PhysicalAddress>()
}

fn test_page_aligned_virtual_address_is_aligned_aligned() -> bool {
    test_is_aligned_aligned::<PageAligned<VirtualAddress>>()
}

fn test_page_aligned_physical_address_is_aligned_aligned() -> bool {
    test_is_aligned_aligned::<PageAligned<PhysicalAddress>>()
}

/// Tests if [`VirtualAddress::is_aligned()`] asserts a non-aligned address as not aligned.
fn test_is_aligned_unaligned<T: Address>() -> bool {
    let raw_addr: usize = PAGE_SIZE;
    let unaligned_address: T = match T::from_raw_value(raw_addr + 1) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match unaligned_address.is_aligned(PAGE_ALIGNMENT) {
        // A non-aligned address was asserted as not aligned.
        Ok(false) => true,
        // A non-aligned address was asserted as aligned.
        Ok(true) => {
            error!("non-aligned address was asserted as aligned (addr={:?})", unaligned_address);
            false
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_is_aligned_unaligned() -> bool {
    test_is_aligned_unaligned::<VirtualAddress>()
}

fn test_physical_address_is_aligned_unaligned() -> bool {
    test_is_aligned_unaligned::<PhysicalAddress>()
}

/// Tests if [`VirtualAddress::is_aligned()`] asserts a mismatched-aligned address as not aligned.
fn test_is_aligned_mismatched_alignment<T: Address>() -> bool {
    let align: Alignment = PGTAB_ALIGNMENT;
    let raw_addr: usize = PAGE_ALIGNMENT as usize;
    let mismatched_aligned_address: T = match T::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match mismatched_aligned_address.is_aligned(align) {
        // A mismatched-aligned address was asserted as not aligned.
        Ok(false) => true,
        // A mismatched-aligned address was asserted as aligned.
        Ok(true) => {
            error!(
                "mismatched-aligned address was asserted as aligned (addr={:?})",
                mismatched_aligned_address
            );
            false
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_is_aligned_mismatched_alignment() -> bool {
    test_is_aligned_mismatched_alignment::<VirtualAddress>()
}

fn test_physical_address_is_aligned_mismatched_alignment() -> bool {
    test_is_aligned_mismatched_alignment::<PhysicalAddress>()
}

fn test_page_aligned_virtual_address_is_aligned_mismatched_alignment() -> bool {
    test_is_aligned_mismatched_alignment::<PageAligned<VirtualAddress>>()
}

fn test_page_aligned_physical_address_is_aligned_mismatched_alignment() -> bool {
    test_is_aligned_mismatched_alignment::<PageAligned<PhysicalAddress>>()
}

/// Tests if [`VirtualAddress::align_up()`] aligns a non-aligned address upwards.
fn test_align_up_unaligned<T: Address>() -> bool {
    let align: Alignment = PAGE_ALIGNMENT;
    let raw_addr: usize = PAGE_SIZE + 1;
    let unaligned_address: T = match T::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match unaligned_address.align_up(align) {
        // The address was aligned upwards.
        Ok(aligned_address) => {
            match aligned_address.is_aligned(align) {
                // The address was aligned.
                Ok(true) => {
                    // The address was aligned upwards.
                    if aligned_address > unaligned_address {
                        true
                    } else {
                        error!(
                            "address was not aligned upwards (unaligned={:?}, aligned={:?})",
                            unaligned_address, aligned_address
                        );
                        false
                    }
                },
                // The address was not aligned.
                Ok(false) => {
                    error!("address was not aligned (addr={:?})", aligned_address);
                    false
                },
                // Some unexpected error occurred.
                Err(err) => {
                    error!("unexpected error (err={:?})", err);
                    false
                },
            }
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            true
        },
    }
}

fn test_virtual_address_align_up_unaligned() -> bool {
    test_align_up_unaligned::<VirtualAddress>()
}

fn test_physical_address_align_up_unaligned() -> bool {
    test_align_up_unaligned::<PhysicalAddress>()
}

/// Tests if [`VirtualAddress::align_up()`] leaves an aligned address as is.
fn test_align_up_aligned<T: Address>() -> bool {
    let align: Alignment = PAGE_ALIGNMENT;
    let raw_addr: usize = PAGE_SIZE;
    let aligned_address: T = match T::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match aligned_address.align_up(align) {
        // The address was aligned upwards.
        Ok(re_aligned_address) => {
            match re_aligned_address.is_aligned(align) {
                // The address was aligned.
                Ok(true) => {
                    // The address was left as is.
                    if re_aligned_address == aligned_address {
                        true
                    } else {
                        error!(
                            "address was not left as is (re-aligned={:?}, aligned={:?})",
                            re_aligned_address, aligned_address
                        );
                        false
                    }
                },
                // The address was not aligned.
                Ok(false) => {
                    error!("address was not aligned (addr={:?})", re_aligned_address);
                    false
                },
                // Some unexpected error occurred.
                Err(err) => {
                    error!("unexpected error (err={:?})", err);
                    false
                },
            }
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_align_up_aligned() -> bool {
    test_align_up_aligned::<VirtualAddress>()
}

fn test_physical_address_align_up_aligned() -> bool {
    test_align_up_aligned::<PhysicalAddress>()
}

fn test_page_aligned_virtual_address_align_up_aligned() -> bool {
    test_align_up_aligned::<PageAligned<VirtualAddress>>()
}

fn test_page_aligned_physical_address_align_up_aligned() -> bool {
    test_align_up_aligned::<PageAligned<PhysicalAddress>>()
}

/// Tests if [`VirtualAddress::align_down()`] aligns a non-aligned address downwards.
fn test_align_down_unaligned<T: Address>() -> bool {
    let align: Alignment = PAGE_ALIGNMENT;
    let raw_addr: usize = PAGE_SIZE - 1;
    let unaligned_address: T = match T::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match unaligned_address.align_down(align) {
        // The address was aligned downwards.
        Ok(aligned_address) => {
            match aligned_address.is_aligned(align) {
                // The address was aligned.
                Ok(true) => {
                    // The address was aligned downwards.
                    if aligned_address < unaligned_address {
                        true
                    } else {
                        error!(
                            "address was not aligned downwards (unaligned={:?}, aligned={:?})",
                            unaligned_address, aligned_address
                        );
                        false
                    }
                },
                // The address was not aligned.
                Ok(false) => {
                    error!("address was not aligned (addr={:?})", aligned_address);
                    false
                },
                // Some unexpected error occurred.
                Err(err) => {
                    error!("unexpected error (err={:?})", err);
                    false
                },
            }
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_align_down_unaligned() -> bool {
    test_align_down_unaligned::<VirtualAddress>()
}

fn test_physical_address_align_down_unaligned() -> bool {
    test_align_down_unaligned::<PhysicalAddress>()
}

/// Tests if [`VirtualAddress::align_down()`] leaves an aligned address as is.
fn test_align_down_aligned<T: Address>() -> bool {
    let align: Alignment = PAGE_ALIGNMENT;
    let raw_addr: usize = PAGE_SIZE;
    let aligned_address: T = match T::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match aligned_address.align_down(align) {
        // The address was aligned downwards.
        Ok(re_aligned_address) => {
            match re_aligned_address.is_aligned(align) {
                // The address was aligned.
                Ok(true) => {
                    // The address was left as is.
                    if re_aligned_address == aligned_address {
                        true
                    } else {
                        error!(
                            "address was not left as is (re-aligned={:?}, aligned={:?})",
                            re_aligned_address, aligned_address
                        );
                        false
                    }
                },
                // The address was not aligned.
                Ok(false) => {
                    error!("address was not aligned (addr={:?})", re_aligned_address);
                    false
                },
                // Some unexpected error occurred.
                Err(err) => {
                    error!("unexpected error (err={:?})", err);
                    false
                },
            }
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

fn test_virtual_address_align_down_aligned() -> bool {
    test_align_down_aligned::<VirtualAddress>()
}

fn test_physical_address_align_down_aligned() -> bool {
    test_align_down_aligned::<PhysicalAddress>()
}

fn test_page_aligned_virtual_address_align_down_aligned() -> bool {
    test_align_down_aligned::<PageAligned<VirtualAddress>>()
}

fn test_page_aligned_physical_address_align_down_aligned() -> bool {
    test_align_down_aligned::<PageAligned<PhysicalAddress>>()
}

/// Tests if [`PageRange`] yields the address of every page in a range, in ascending order.
fn test_page_range_yields_every_page() -> bool {
    let npages: usize = 3;
    let raw_start: usize = PAGE_SIZE;
    let start: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(raw_start) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    let pages: PageRange = match PageRange::new(start, npages * PAGE_SIZE) {
        Ok(pages) => pages,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create page range (err={:?})", err);
            return false;
        },
    };

    let raw_end: usize = raw_start + npages * PAGE_SIZE;
    let mut expected: usize = raw_start;
    for page in pages {
        // Check if the range yielded a page other than the expected one.
        if expected == raw_end || page.into_raw_value() != expected {
            error!("unexpected page (page={:?}, expected={:#x})", page, expected);
            return false;
        }
        expected += PAGE_SIZE;
    }

    // Check if the range did not yield every page.
    if expected != raw_end {
        error!("missing pages (next={:#x}, end={:#x})", expected, raw_end);
        return false;
    }

    true
}

/// Tests if [`PageRange`] yields no page for a range of zero bytes.
fn test_page_range_empty() -> bool {
    let start: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(PAGE_SIZE) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match PageRange::new(start, 0) {
        Ok(mut pages) => match pages.next() {
            // The range is empty as expected.
            None => true,
            // The range yielded a page and it should not.
            Some(page) => {
                error!("empty range yielded a page (page={:?})", page);
                false
            },
        },
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create page range (err={:?})", err);
            false
        },
    }
}

/// Tests if [`PageRange`] yields the last page of the virtual address space.
fn test_page_range_last_page() -> bool {
    let raw_addr: usize = VirtualAddress::max_addr() & !(PAGE_SIZE - 1);
    let start: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    let mut pages: PageRange = match PageRange::new(start, PAGE_SIZE) {
        Ok(pages) => pages,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create page range (err={:?})", err);
            return false;
        },
    };

    match pages.next() {
        // The range yielded the last page as expected.
        Some(page) if page == start => {},
        // The range yielded some other page, or no page at all.
        page => {
            error!("unexpected page (page={:?}, expected={:?})", page, start);
            return false;
        },
    }

    match pages.next() {
        // The range yielded no other page as expected.
        None => true,
        // The range yielded a page past the end of the virtual address space.
        Some(page) => {
            error!("range yielded too many pages (page={:?})", page);
            false
        },
    }
}

/// Tests if [`PageRange::new()`] fails to create a range that does not span a whole number of
/// pages.
fn test_page_range_new_unaligned_size() -> bool {
    let start: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(PAGE_SIZE) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match PageRange::new(start, PAGE_SIZE + 1) {
        // The range was created and it should not.
        Ok(pages) => {
            error!("succeeded to create a range with an unaligned size (pages={:?})", pages);
            false
        },
        // The range was not created as expected.
        Err(e) if e.code == ErrorCode::InvalidArgument => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

/// Tests if [`PageRange::new()`] fails to create a range that extends beyond the end of the virtual
/// address space.
fn test_page_range_new_overflow() -> bool {
    let raw_addr: usize = VirtualAddress::max_addr() & !(PAGE_SIZE - 1);
    let start: PageAligned<VirtualAddress> = match PageAligned::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match PageRange::new(start, 2 * PAGE_SIZE) {
        // The range was created and it should not.
        Ok(pages) => {
            error!("succeeded to create an overflowing range (pages={:?})", pages);
            false
        },
        // The range was not created as expected.
        Err(e) if e.code == ErrorCode::ValueOverflow => true,
        // Some unexpected error occurred.
        Err(err) => {
            error!("unexpected error (err={:?})", err);
            false
        },
    }
}

/// Runs all unit tests for [`VirtualAddress`].
pub fn test() -> bool {
    let mut passed: bool = true;

    // Tests for `VirtualAddress`.
    passed &= run_test!(test_virtual_address_new_zero_address);
    passed &= run_test!(test_virtual_address_new_max_address);
    // No test_virtual_address_new_out_of_range_address
    passed &= run_test!(test_virtual_address_is_aligned_aligned);
    passed &= run_test!(test_virtual_address_is_aligned_unaligned);
    passed &= run_test!(test_virtual_address_is_aligned_mismatched_alignment);
    passed &= run_test!(test_virtual_address_align_up_unaligned);
    passed &= run_test!(test_virtual_address_align_up_aligned);
    passed &= run_test!(test_virtual_address_align_down_unaligned);
    passed &= run_test!(test_virtual_address_align_down_aligned);

    // Tests for `PhysicalAddress`.
    passed &= run_test!(test_physical_address_new_zero_address);
    passed &= run_test!(test_physical_address_new_max_address);
    passed &= run_test!(test_physical_address_new_out_of_range_address);
    passed &= run_test!(test_physical_address_is_aligned_aligned);
    passed &= run_test!(test_physical_address_is_aligned_unaligned);
    passed &= run_test!(test_physical_address_is_aligned_mismatched_alignment);
    passed &= run_test!(test_physical_address_align_up_unaligned);
    passed &= run_test!(test_physical_address_align_up_aligned);
    passed &= run_test!(test_physical_address_align_down_unaligned);
    passed &= run_test!(test_physical_address_align_down_aligned);

    // Tests for `PageAligned<VirtualAddress>`.
    passed &= run_test!(test_page_aligned_virtual_address_new_zero_address);
    passed &= run_test!(test_page_aligned_virtual_address_new_max_address);
    // No test_page_aligned_virtual_address_new_out_of_range_address: VirtualAddress covers full
    // usize range, so max_addr() + 1 overflows to 0 (valid). Out-of-range is not applicable.
    passed &= run_test!(test_page_aligned_virtual_address_new_unaligned);
    passed &= run_test!(test_page_aligned_virtual_address_is_aligned_aligned);
    // No test_page_aligned_virtual_address_is_aligned_unaligned: PageAligned rejects unaligned
    // addresses at construction, so an unaligned instance cannot exist.
    passed &= run_test!(test_page_aligned_virtual_address_is_aligned_mismatched_alignment);
    // No test_page_aligned_virtual_address_align_up_unaligned: same reason as above.
    passed &= run_test!(test_page_aligned_virtual_address_align_up_aligned);
    // No test_page_aligned_virtual_address_align_down_unaligned: same reason as above.
    passed &= run_test!(test_page_aligned_virtual_address_align_down_aligned);

    // Tests for `PageAligned<PhysicalAddress>`.
    passed &= run_test!(test_page_aligned_physical_address_new_zero_address);
    passed &= run_test!(test_page_aligned_physical_address_new_max_address);
    passed &= run_test!(test_page_aligned_physical_address_new_out_of_range_address);
    passed &= run_test!(test_page_aligned_physical_address_new_unaligned);
    passed &= run_test!(test_page_aligned_physical_address_is_aligned_aligned);
    // No test_page_aligned_physical_address_is_aligned_unaligned: PageAligned rejects unaligned
    // addresses at construction, so an unaligned instance cannot exist.
    passed &= run_test!(test_page_aligned_physical_address_is_aligned_mismatched_alignment);
    // No test_page_aligned_physical_address_align_up_unaligned: same reason as above.
    passed &= run_test!(test_page_aligned_physical_address_align_up_aligned);
    // No test_page_aligned_physical_address_align_down_unaligned: same reason as above.
    passed &= run_test!(test_page_aligned_physical_address_align_down_aligned);

    // Tests for `PageRange`.
    passed &= run_test!(test_page_range_yields_every_page);
    passed &= run_test!(test_page_range_empty);
    passed &= run_test!(test_page_range_last_page);
    passed &= run_test!(test_page_range_new_unaligned_size);
    passed &= run_test!(test_page_range_new_overflow);

    passed
}
