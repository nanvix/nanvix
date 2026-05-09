// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Hyperlight GPA→GVA Translation Backend
//!
//! On Hyperlight, GVA ≠ GPA for scratch-region frames. Every physical address must be translated
//! to a guest virtual address via [`gpa_to_gva`](crate::hal::platform::gpa_to_gva) before the CPU
//! can access it. The translation offset differs between the snapshot region (identity, GVA == GPA)
//! and the scratch region (constant delta), so a multi-page operation that crosses a region boundary
//! requires a fresh translation at each page. This backend processes copies and fills one page at a
//! time to guarantee correctness regardless of the physical layout.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    arch::x86::{
        fast_memcpy,
        fast_memset,
    },
    platform::gpa_to_gva,
};
use ::arch::mem::{
    self,
    PAGE_ALIGNMENT,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::align_down,
};

//==================================================================================================
// Public API
//==================================================================================================

/// Copies bytes between two physical memory regions, translating GPAs to GVAs page-by-page.
pub(crate) fn memcpy(dst: *mut u8, src: *const u8, size: usize) -> Result<(), Error> {
    if size == 0 {
        return Ok(());
    }

    let dst_start: usize = dst as usize;
    let src_start: usize = src as usize;
    let dst_end: usize = dst_start.checked_add(size).ok_or_else(|| {
        error!("memcpy(): destination range overflows (dst={dst_start:#x}, size={size:#x})");
        Error::new(ErrorCode::BadAddress, "memcpy(): destination range overflows")
    })?;
    let src_end: usize = src_start.checked_add(size).ok_or_else(|| {
        error!("memcpy(): source range overflows (src={src_start:#x}, size={size:#x})");
        Error::new(ErrorCode::BadAddress, "memcpy(): source range overflows")
    })?;

    // Check if copy ranges overlap.
    if (dst_start..dst_end).contains(&src_start) || (src_start..src_end).contains(&dst_start) {
        error!(
            "memcpy(): source and destination ranges overlap (dst={dst_start:#x}, \
             src={src_start:#x}, size={size:#x})"
        );
        return Err(Error::new(
            ErrorCode::BadAddress,
            "memcpy(): source and destination ranges overlap",
        ));
    }

    // Copy page-by-page, translating GPA→GVA at each page boundary so that region
    // crossings (snapshot ↔ scratch) are handled correctly.
    let mut src_gpa: usize = src as usize;
    let mut dst_gpa: usize = dst as usize;
    let mut remaining: usize = size;

    while remaining > 0 {
        // The chunk size is the minimum of: bytes left, bytes to end of src page,
        // and bytes to end of dst page.
        let src_page_rem: usize = mem::PAGE_SIZE - (src_gpa - align_down(src_gpa, PAGE_ALIGNMENT));
        let dst_page_rem: usize = mem::PAGE_SIZE - (dst_gpa - align_down(dst_gpa, PAGE_ALIGNMENT));
        let chunk: usize = remaining.min(src_page_rem).min(dst_page_rem);

        let src_gva: *const u8 = gpa_to_gva(src_gpa) as *const u8;
        let dst_gva: *mut u8 = gpa_to_gva(dst_gpa) as *mut u8;

        // SAFETY: the host page tables map both GVAs to valid physical frames after
        // eager pre-faulting. The overlap check above guarantees non-overlapping ranges.
        // The chunk never exceeds PAGE_SIZE, so no page boundary is crossed.
        unsafe { fast_memcpy(dst_gva, src_gva, chunk) };

        src_gpa += chunk;
        dst_gpa += chunk;
        remaining -= chunk;
    }

    Ok(())
}

/// Fills bytes in a physical memory range, translating GPAs to GVAs page-by-page.
pub(crate) fn memset(base: *mut u8, value: u8, size: usize) -> Result<(), Error> {
    if size == 0 {
        return Ok(());
    }

    let base_start: usize = base as usize;
    base_start.checked_add(size).ok_or_else(|| {
        error!("memset(): target range overflows (base={base_start:#x}, size={size:#x})");
        Error::new(ErrorCode::BadAddress, "memset(): target range overflows")
    })?;

    // Fill page-by-page, translating GPA→GVA at each page boundary.
    let mut gpa: usize = base as usize;
    let mut remaining: usize = size;

    while remaining > 0 {
        let page_rem: usize = mem::PAGE_SIZE - (gpa - align_down(gpa, PAGE_ALIGNMENT));
        let chunk: usize = remaining.min(page_rem);
        let gva: *mut u8 = gpa_to_gva(gpa) as *mut u8;

        // SAFETY: the host page tables map the GVA to a valid physical frame after
        // eager pre-faulting.
        unsafe { fast_memset(gva, value, chunk) };

        gpa += chunk;
        remaining -= chunk;
    }

    Ok(())
}

/// Executes a closure. On Hyperlight, the host-bootstrapped page tables are always active and the
/// kernel does not manage its own CR3, so this is a trivial passthrough.
pub(crate) fn with_kernel_address_space<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    f()
}
