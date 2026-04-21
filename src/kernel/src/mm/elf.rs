// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Exceptions
//==================================================================================================

// Not all functions are used.
#![allow(dead_code)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        AccessPermission,
        ExecutePermission,
        PageAligned,
        ReadPermission,
        VirtualAddress,
        WritePermission,
    },
    mm::{
        phys::UserFrame,
        VirtMemoryManager,
        Vmem,
    },
};
use ::alloc::vec::Vec;
use ::arch::{
    mem,
    mem::PAGE_ALIGNMENT,
};
use ::core::cmp::{
    max,
    min,
};
pub use ::elf::elf32::Elf32Fhdr;
use ::elf::elf32::{
    Elf32Phdr,
    ET_DYN,
    ET_EXEC,
    PF_R,
    PF_W,
    PF_X,
    PT_LOAD,
};
use ::sys::{
    config::memory_layout::{
        USER_BASE,
        USER_BASE_RAW,
    },
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        Alignment,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

// Maximum number of PT_LOAD segments tracked for overlap detection. A small fixed-size
// array is used instead of a heap-allocated map because real-world ELF binaries rarely
// have more than a handful of LOAD segments, making linear scans faster and avoiding
// kernel heap allocation overhead. Do not increase this value without revisiting the
// overlap detection algorithm: it performs an O(segments) scan per page, which is only
// efficient for small segment counts.
const MAX_LOAD_SEGMENTS: usize = 16;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the union of two access permissions, allowing any access granted by either operand.
///
/// # Parameters
///
/// - `a`: First access permission.
/// - `b`: Second access permission.
///
/// # Returns
///
/// An [`AccessPermission`] that grants read, write, or execute access whenever at least one of
/// the two operands grants it.
///
fn merge_access(a: AccessPermission, b: AccessPermission) -> AccessPermission {
    AccessPermission::new(
        if a.is_readable() || b.is_readable() {
            ReadPermission::Allow
        } else {
            ReadPermission::Deny
        },
        if a.is_writable() || b.is_writable() {
            WritePermission::Allow
        } else {
            WritePermission::Deny
        },
        if a.is_executable() || b.is_executable() {
            ExecutePermission::Allow
        } else {
            ExecutePermission::Deny
        },
    )
}

///
/// # Description
///
/// Loads an ELF32 binary into a target virtual memory space.
///
/// # Parameters
///
/// - `mm`: Virtual memory manager.
/// - `vmem`: Target virtual memory space.
/// - `elf`: ELF32 file header.
///
/// # Returns
///
/// Upon successful completion, the entry point of the ELF32 binary and the address past the
/// last loaded segment are returned. If an error occurs, an error code is returned and the
/// virtual memory space may be left in an inconsistent state.
///
fn do_elf32_load(
    mm: &mut VirtMemoryManager,
    vmem: &mut Vmem,
    elf: &Elf32Fhdr,
    dry_run: bool,
) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> {
    trace!("dry_run={}", dry_run);

    let mut last_address: usize = 0;

    // Page-aligned ranges and original permissions of already-processed LOAD segments.  This is
    // used to detect overlapping pages (e.g., a RELRO segment sharing a page with the preceding
    // text segment) and widen permissions instead of double-allocating.  Entries are never mutated
    // after insertion so that the union is always computed from per-segment originals, preventing
    // permission bleed across non-overlapping pages.
    let mut loaded_ranges: [(usize, usize, AccessPermission); MAX_LOAD_SEGMENTS] =
        [(0, 0, AccessPermission::RDONLY); MAX_LOAD_SEGMENTS];
    let mut loaded_count: usize = 0;

    // Check if the ELF header is valid.
    if let Err(reason) = elf.validate() {
        error!("{reason}");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    // SAFETY: `e_phoff` is the byte offset from the ELF header to the program header table.
    // The resulting pointer is within the ELF image, which the caller guarantees is valid.
    let phdr_base: *const Elf32Phdr = unsafe {
        (elf as *const Elf32Fhdr as *const u8).offset(elf.e_phoff as isize) as *const Elf32Phdr
    };
    // SAFETY: `e_phnum` entries starting at `phdr_base` are guaranteed to reside within the
    // ELF image. Each entry is a `repr(C)` `Elf32Phdr` with no invalid bit patterns.
    let phdrs: &[Elf32Phdr] =
        unsafe { core::slice::from_raw_parts(phdr_base, elf.e_phnum as usize) };

    // Only ET_EXEC and ET_DYN binaries are supported.
    if elf.e_type != ET_EXEC && elf.e_type != ET_DYN {
        let reason: &str = "unsupported ELF type";
        error!("{reason} (e_type={:#x})", elf.e_type);
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    // Compute load base for PIE (ET_DYN) binaries. If the lowest PT_LOAD virtual address is
    // below USER_BASE, offset all segment addresses so they land in user space.
    let load_base: usize = if elf.e_type == ET_DYN {
        let user_base: usize = USER_BASE_RAW;
        let lowest_vaddr: usize = phdrs
            .iter()
            .filter(|phdr| phdr.p_type == PT_LOAD)
            .map(|phdr| phdr.p_vaddr as usize)
            .min()
            .unwrap_or(0);
        if lowest_vaddr < user_base {
            user_base.saturating_sub(lowest_vaddr)
        } else {
            0
        }
    } else {
        0
    };

    let entry_raw: usize = (elf.e_entry as usize)
        .checked_add(load_base)
        .ok_or_else(|| {
            let reason: &str = "entry address overflow";
            error!("{reason} (e_entry={:#x}, load_base={:#x})", elf.e_entry, load_base);
            Error::new(ErrorCode::BadFile, reason)
        })?;
    let entry: VirtualAddress = VirtualAddress::new(entry_raw);

    // Check if entry point does not match what we expect.
    if entry < USER_BASE {
        let reason: &str = "invalid binary entry point";
        error!("{} (entry={:?})", reason, entry);
        return Err(Error::new(ErrorCode::BadFile, "invalid entry point"));
    }

    // Load segments.
    for phdr in phdrs {
        if !phdr.is_loadable() {
            continue;
        }

        // Check if the segment is not valid.
        if let Err(reason) = phdr.validate() {
            error!("{reason}");
            return Err(Error::new(ErrorCode::BadFile, reason));
        }

        let align: Alignment = phdr
            .p_align
            .try_into()
            .map_err(|_| Error::new(ErrorCode::BadFile, "invalid alignment value in elf file"))?;
        let adjusted_vaddr: usize =
            (phdr.p_vaddr as usize)
                .checked_add(load_base)
                .ok_or_else(|| {
                    let reason: &str = "virtual address overflow in PIE segment";
                    error!("{reason} (p_vaddr={:#x}, load_base={load_base:#x})", phdr.p_vaddr);
                    Error::new(ErrorCode::BadFile, reason)
                })?;
        let virt_addr_base: usize = ::sys::mm::align_down(adjusted_vaddr, align);

        // Compute access permissions.
        let access: AccessPermission = if phdr.p_flags == (PF_R | PF_X) {
            AccessPermission::EXEC
        } else if (phdr.p_flags & PF_W) != 0 {
            AccessPermission::RDWR
        } else {
            AccessPermission::RDONLY
        };

        // Allocate segment.
        let size: usize = max(phdr.p_filesz as usize, phdr.p_memsz as usize);
        let virt_addr_range_end: usize = adjusted_vaddr.checked_add(size).ok_or_else(|| {
            let reason: &str = "virtual address overflow in elf segment";
            error!("{reason} (adjusted_vaddr={adjusted_vaddr:#x}, size={size})");
            Error::new(ErrorCode::BadFile, reason)
        })?;
        let virt_addr_end: usize = ::sys::mm::align_up(virt_addr_range_end, PAGE_ALIGNMENT)
            .ok_or_else(|| {
                let reason: &str = "align_up overflow";
                error!("{reason} (virt_addr_range_end={virt_addr_range_end:#x})");
                Error::new(ErrorCode::BadFile, reason)
            })?;

        // SAFETY: `p_offset` is the byte offset from the start of the ELF image to the
        // segment data. The caller guarantees the ELF image spans at least this range.
        let phys_addr_base: usize = unsafe {
            (elf as *const Elf32Fhdr as *const u8).offset(phdr.p_offset as isize) as usize
        };

        let phys_addr_end: usize = phys_addr_base + phdr.p_filesz as usize;

        // Load segment page by page.
        debug!(
            "loading segment (virt_addr_base={:#x}, virt_addr_end={:#x}, phys_addr_base={:#x}, \
             phys_addr_end={:#x}, access={:?})",
            virt_addr_base, virt_addr_end, phys_addr_base, phys_addr_end, access
        );

        let mut uframe_buf: Vec<UserFrame> = Vec::with_capacity(1);
        for vaddr in (virt_addr_base..virt_addr_end).step_by(mem::PAGE_SIZE) {
            let vaddr: VirtualAddress = VirtualAddress::new(vaddr);

            // Check if address lies in user space.
            if vaddr < USER_BASE {
                let reason: &str = "invalid load address";
                error!("{reason}");
                return Err(Error::new(ErrorCode::BadFile, reason));
            }

            let vaddr: PageAligned<VirtualAddress> = PageAligned::from_address(vaddr)?;

            // Check if we should perform the allocation.
            if !dry_run {
                let page_addr: usize = vaddr.into_raw_value();

                // Scan prior segment ranges to detect overlap and compute the merged
                // permission from all segments that cover this page.
                let mut already_mapped: bool = false;
                let mut merged: AccessPermission = access;
                for &(start, end, prev_access) in loaded_ranges.iter().take(loaded_count) {
                    if page_addr >= start && page_addr < end {
                        merged = merge_access(merged, prev_access);
                        already_mapped = true;
                    }
                }

                if already_mapped {
                    // Page already mapped by a prior segment — apply merged permissions
                    // to accommodate all segments sharing this page.
                    mm.ctrl_upage(vmem, vaddr, merged)?;
                } else {
                    // Only clear pages that will NOT be fully overwritten by segment data.
                    // A page is fully covered when the segment data for this page spans
                    // the entire PAGE_SIZE bytes; in that case clearing is redundant.

                    // Start of the physical/source-backed data for this page.
                    let page_offset_in_segment: usize = vaddr.into_raw_value() - virt_addr_base;
                    let page_phys_addr: usize =
                        match phys_addr_base.checked_add(page_offset_in_segment) {
                            Some(addr) => addr,
                            None => {
                                let reason: &str = "invalid physical address";
                                error!("{reason}");
                                return Err(Error::new(ErrorCode::BadFile, reason));
                            },
                        };
                    // One-past-the-end if the full page were backed by segment data.
                    let page_phys_addr_end: usize = match page_phys_addr.checked_add(mem::PAGE_SIZE)
                    {
                        Some(end) => end,
                        None => {
                            let reason: &str = "invalid physical address range";
                            error!("{reason}");
                            return Err(Error::new(ErrorCode::BadFile, reason));
                        },
                    };

                    // Page is entirely beyond segment data (pure BSS) — must be zeroed.
                    let page_lies_in_bss: bool = page_phys_addr >= phys_addr_end;
                    // Page straddles the segment-data/BSS boundary — trailing bytes must
                    // be zeroed.
                    let page_is_partially_covered: bool =
                        page_phys_addr < phys_addr_end && page_phys_addr_end > phys_addr_end;
                    mm.alloc_upages(
                        vmem,
                        vaddr,
                        access,
                        page_lies_in_bss || page_is_partially_covered,
                        &mut uframe_buf,
                    )?;
                }
            }

            // Update last address.
            if vaddr.into_raw_value() + mem::PAGE_SIZE > last_address {
                last_address = vaddr.into_raw_value() + mem::PAGE_SIZE;
            }

            let phys_addr: usize = phys_addr_base + (vaddr.into_raw_value() - virt_addr_base);

            // Load segment only if it is within bounds.
            if phys_addr < phys_addr_end {
                let size: usize = min(mem::PAGE_SIZE, phys_addr_end - phys_addr);

                // Load segment only if it has a non-zero size.
                if size > 0 {
                    vmem.copy_to_user_unaligned_unchecked(
                        vaddr.into_inner(),
                        VirtualAddress::from_raw_value(phys_addr),
                        size,
                        dry_run,
                    )?;
                }
            }
        }

        // Record this segment's page-aligned range so subsequent segments can detect overlap.
        // If the array is full, skip recording — overlap detection degrades gracefully by
        // not merging permissions for segments beyond the limit.
        if loaded_count < MAX_LOAD_SEGMENTS {
            loaded_ranges[loaded_count] = (virt_addr_base, virt_addr_end, access);
            loaded_count += 1;
        } else {
            warn!("too many load segments, overlap detection may be inaccurate");
        }
    }

    let aligned_last: VirtualAddress = VirtualAddress::new(last_address)
        .align_up(PAGE_ALIGNMENT)
        .ok_or_else(|| {
        let reason: &str = "align_up overflow";
        error!("{reason} (last_address={last_address:#x})");
        Error::new(ErrorCode::BadFile, reason)
    })?;

    Ok((entry, PageAligned::from_address(aligned_last)?))
}

pub fn elf32_load(
    mm: &mut VirtMemoryManager,
    vmem: &mut Vmem,
    elf: &Elf32Fhdr,
) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> {
    if cfg!(feature = "nightly-performance-optimizations") {
        do_elf32_load(mm, vmem, elf, false)
    } else {
        // Two-pass: first validate with a dry run, then load.
        do_elf32_load(mm, vmem, elf, true)?;
        do_elf32_load(mm, vmem, elf, false)
    }
}
