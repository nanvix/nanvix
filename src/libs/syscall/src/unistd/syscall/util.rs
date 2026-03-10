// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::core::cmp;

#[cfg(feature = "ring-buffer")]
const MAX_RING_TRANSFER_SIZE: usize = 16 * PAGE_SIZE;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the number of bytes that can be transferred starting at `ptr` without crossing a page
/// boundary. The kernel's data chunk transfer path (push/pull) translates only the first page's virtual
/// address to a guest physical address, so each individual transfer must be contained within a
/// single physical page. This function ensures that constraint by clamping the transfer size to
/// the remaining bytes on the current page.
///
/// # Parameters
///
/// - `ptr`: Start address of the buffer.
/// - `remaining`: Total number of bytes remaining to transfer.
///
/// # Returns
///
/// The number of bytes that fit within the current page.
///
#[cfg(not(feature = "ring-buffer"))]
pub fn page_chunk_size(ptr: usize, remaining: usize) -> usize {
    let page_offset: usize = ptr & (PAGE_SIZE - 1);
    let available: usize = PAGE_SIZE - page_offset;
    cmp::min(available, remaining)
}

/// Computes the maximum byte count that should be forwarded in a single syscall-library request.
///
/// On the legacy transport this preserves the page-boundary restriction of the data-chunk path.
/// On ring-buffer-enabled guest builds the kernel can gather/scatter across user pages, so
/// requests are instead capped by the logical multi-buffer transfer limit.
pub fn transfer_chunk_size(ptr: usize, remaining: usize) -> usize {
    #[cfg(feature = "ring-buffer")]
    {
        let _ = ptr;
        cmp::min(MAX_RING_TRANSFER_SIZE, remaining)
    }

    #[cfg(not(feature = "ring-buffer"))]
    {
        page_chunk_size(ptr, remaining)
    }
}
