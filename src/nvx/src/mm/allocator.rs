// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::alloc::{
    GlobalAlloc,
    Layout,
};
use ::core::ptr;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::talc::*;

//==================================================================================================
//  Structures
//==================================================================================================

struct Allocator;

//==================================================================================================
// Global Variables
//==================================================================================================

static mut HEAP: Option<Talck<spin::Mutex<()>, ClaimOnOom>> = None;

#[global_allocator]
static mut ALLOCATOR: Allocator = Allocator;

//==================================================================================================
// Implementations
//==================================================================================================

///
/// # Description
///
/// Initializes the heap.
///
/// # Parameters
///
/// - `addr` - Start address of the heap.
/// - `size` - Size of the heap.
///
/// # Returns
///
/// Upon success, empty is returned. Upon failure, an error is returned instead
///
#[allow(static_mut_refs)]
pub unsafe fn init(addr: usize, size: usize) -> Result<(), Error> {
    // Check if the heap was already initialized.
    if unsafe { HEAP.is_some() } {
        return Err(Error::new(ErrorCode::ResourceBusy, "heap already initialized"));
    }

    HEAP = Some(Talc::new(ClaimOnOom::new(Span::from_base_size(addr as *mut u8, size))).lock());

    Ok(())
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heap = ptr::addr_of_mut!(HEAP);
        if let Some(heap) = &mut *heap {
            heap.alloc(layout)
        } else {
            // Heap is not initialized.
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let heap = ptr::addr_of_mut!(HEAP);
        if let Some(heap) = &mut *heap {
            heap.dealloc(ptr, layout)
        }
    }
}
