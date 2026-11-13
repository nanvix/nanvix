// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mm::try_box;
use ::alloc::boxed::Box;
use ::config::kernel::MAX_SLAB_SIZE;
use ::core::{
    mem,
    ptr,
};
use ::sys::error::ErrorCode;

//==================================================================================================
// Structures
//==================================================================================================

/// Zero-sized value, for which [`try_box()`] must not allocate memory.
#[derive(PartialEq)]
struct ZeroSized;

/// Value aligned more strictly than any primitive type.
#[repr(align(64))]
struct OverAligned([u8; 64]);

//==================================================================================================
// Tests
//==================================================================================================

/// Tests if [`try_box()`] boxes a zero-sized value.
fn test_try_box_zero_sized() -> bool {
    match try_box(ZeroSized) {
        Ok(boxed) => *boxed == ZeroSized,
        Err(err) => {
            error!("failed to box zero-sized value (err={:?})", err);
            false
        },
    }
}

/// Tests if [`try_box()`] moves a value to kernel heap memory that is aligned for its type.
fn test_try_box_over_aligned() -> bool {
    const PATTERN: u8 = 0xa5;

    let boxed: Box<OverAligned> = match try_box(OverAligned([PATTERN; 64])) {
        Ok(boxed) => boxed,
        Err(err) => {
            error!("failed to box over-aligned value (err={:?})", err);
            return false;
        },
    };

    let addr: usize = ptr::from_ref::<OverAligned>(&boxed).addr();
    if !addr.is_multiple_of(mem::align_of::<OverAligned>()) {
        error!("boxed value is misaligned (addr={:#x})", addr);
        return false;
    }

    if boxed.0.iter().any(|byte| *byte != PATTERN) {
        error!("boxed value does not match the original value");
        return false;
    }

    true
}

/// Tests if [`try_box()`] returns [`ErrorCode::OutOfMemory`] when the kernel heap cannot allocate a
/// value, instead of aborting.
fn test_try_box_out_of_memory() -> bool {
    // The kernel heap does not serve allocations larger than its largest slab.
    match try_box([0u8; MAX_SLAB_SIZE + 1]) {
        Ok(_) => {
            error!("boxed a value larger than the largest kernel heap slab");
            false
        },
        Err(err) => err.code == ErrorCode::OutOfMemory,
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all in-kernel tests of the fallible kernel heap allocation helpers.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= run_test!(test_try_box_zero_sized);
    passed &= run_test!(test_try_box_over_aligned);
    passed &= run_test!(test_try_box_out_of_memory);

    passed
}
