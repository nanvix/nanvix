// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::core::cmp;

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
pub fn page_chunk_size(ptr: usize, remaining: usize) -> usize {
    let page_offset: usize = ptr & (PAGE_SIZE - 1);
    let available: usize = PAGE_SIZE - page_offset;
    cmp::min(available, remaining)
}
