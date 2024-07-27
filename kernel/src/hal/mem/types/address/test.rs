// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    error::ErrorCode,
    hal::{
        arch::x86::mem::mmu,
        mem::{
            Address,
            PageAligned,
            PhysicalAddress,
            VirtualAddress,
        },
    },
    klib::Alignment,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests if [`Address::new()`] successfully instantiates address zero.
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

/// Tests if [`Address::new()`] successfully instantiates the maximum address.
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

/// Tests if [`Address::new()`] fails to create an out of range address.
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

/// Tests if [`PageAligned::new()`] fails to create an unaligned virtual address.
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

/// Tests if [`PageAligned::new()`] fails to create an unaligned physical address.
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
    let raw_addr: usize = mem::PAGE_SIZE;
    let aligned_address: T = match T::from_raw_value(raw_addr) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match aligned_address.is_aligned(mmu::PAGE_ALIGNMENT) {
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
    let raw_addr: usize = mem::PAGE_SIZE;
    let unaligned_address: T = match T::from_raw_value(raw_addr + 1) {
        Ok(addr) => addr,
        // Some unexpected error occurred.
        Err(err) => {
            error!("failed to create address (err={:?})", err);
            return false;
        },
    };
    match unaligned_address.is_aligned(mmu::PAGE_ALIGNMENT) {
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
    let align: Alignment = mmu::PGTAB_ALIGNMENT;
    let raw_addr: usize = mmu::PAGE_ALIGNMENT as usize;
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
    let align: Alignment = mmu::PAGE_ALIGNMENT;
    let raw_addr: usize = mem::PAGE_SIZE + 1;
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
    let align: Alignment = mmu::PAGE_ALIGNMENT;
    let raw_addr: usize = mem::PAGE_SIZE;
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
    let align: Alignment = mmu::PAGE_ALIGNMENT;
    let raw_addr: usize = mem::PAGE_SIZE - 1;
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
    let align: Alignment = mmu::PAGE_ALIGNMENT;
    let raw_addr: usize = mem::PAGE_SIZE;
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
    // passed &= run_test!(test_page_aligned_virtual_address_new_max_address);
    // passed &= run_test!(test_page_aligned_virtual_address_new_out_of_range_address);
    passed &= run_test!(test_page_aligned_virtual_address_new_unaligned);
    passed &= run_test!(test_page_aligned_virtual_address_is_aligned_aligned);
    // No test_page_aligned_virtual_address_is_aligned_unaligned
    passed &= run_test!(test_page_aligned_virtual_address_is_aligned_mismatched_alignment);
    // No test_page_aligned_virtual_address_align_up_unaligned
    passed &= run_test!(test_page_aligned_virtual_address_align_up_aligned);
    // No test_page_aligned_virtual_address_align_down_unaligned
    passed &= run_test!(test_page_aligned_virtual_address_align_down_aligned);

    // Tests for `PageAligned<PhysicalAddress>`.
    passed &= run_test!(test_page_aligned_physical_address_new_zero_address);
    // passed &= run_test!(test_page_aligned_physical_address_new_max_address);
    // passed &= run_test!(test_page_aligned_physical_address_new_out_of_range_address);
    passed &= run_test!(test_page_aligned_physical_address_new_unaligned);
    passed &= run_test!(test_page_aligned_physical_address_is_aligned_aligned);
    // No test_page_aligned_physical_address_is_aligned_unaligned
    passed &= run_test!(test_page_aligned_physical_address_is_aligned_mismatched_alignment);
    // No test_page_aligned_physical_address_align_up_unaligned
    passed &= run_test!(test_page_aligned_physical_address_align_up_aligned);
    // No test_page_aligned_physical_address_align_down_unaligned
    passed &= run_test!(test_page_aligned_physical_address_align_down_aligned);

    // TODO: test_get_pte_index
    // TODO: test_get_pde_index
    // TODO: make tests generic over `Address`.

    passed
}
