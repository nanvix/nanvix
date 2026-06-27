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
use ::core::{
    cmp::{
        max,
        min,
    },
    mem::MaybeUninit,
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

//==================================================================================================
// ELF Image Source
//==================================================================================================

///
/// # Description
///
/// Describes where the loader reads ELF image bytes from.
///
/// The boot path loads from a contiguous, kernel-addressable image (the initrd). The `execv()`
/// path loads directly from the calling process's address space, where the image was placed by
/// `mmap` and may be physically non-contiguous; reading it byte-for-byte into a contiguous kernel
/// buffer would impose an artificial size limit, so the loader streams it page-by-page instead.
///
#[derive(Clone, Copy)]
enum ElfSource<'a> {
    /// Contiguous, kernel-addressable ELF image whose first byte is at kernel address `base`
    /// (boot/initrd path). Trusted: no length bound is enforced.
    Blob { base: usize },
    /// ELF image resident in user address space `vmem`, starting at virtual address `base` and
    /// spanning `len` bytes (execv path). May be physically non-contiguous. Untrusted: every read
    /// is bounds-checked against `len`.
    User {
        vmem: &'a Vmem,
        base: VirtualAddress,
        len: usize,
    },
}

impl ElfSource<'_> {
    /// Returns the byte length of the image, if it is bounded.
    fn len_limit(&self) -> Option<usize> {
        match self {
            ElfSource::Blob { .. } => None,
            ElfSource::User { len, .. } => Some(*len),
        }
    }

    /// Validates that the half-open range `[offset, offset + size)` lies within the image bounds
    /// (only enforced for bounded sources).
    fn check_bounds(&self, offset: usize, size: usize) -> Result<(), Error> {
        if let Some(limit) = self.len_limit() {
            let end: usize = offset
                .checked_add(size)
                .ok_or_else(|| Error::new(ErrorCode::BadFile, "elf offset overflow"))?;
            if end > limit {
                let reason: &str = "elf read out of bounds";
                error!("{reason} (offset={offset}, size={size}, limit={limit})");
                return Err(Error::new(ErrorCode::BadFile, reason));
            }
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Reads `size` bytes at file `offset` within the image into the kernel buffer `dst`.
    ///
    /// # Safety
    ///
    /// `dst` must point to a writable kernel buffer of at least `size` bytes.
    ///
    fn read_bytes(&self, offset: usize, dst: *mut u8, size: usize) -> Result<(), Error> {
        if size == 0 {
            return Ok(());
        }
        self.check_bounds(offset, size)?;
        match self {
            ElfSource::Blob { base } => {
                let src: usize = base
                    .checked_add(offset)
                    .ok_or_else(|| Error::new(ErrorCode::BadFile, "elf blob offset overflow"))?;
                // SAFETY: the boot ELF image is contiguous and identity-mapped in the kernel
                // address space, and `dst` is a distinct kernel buffer of `size` bytes.
                unsafe { core::ptr::copy_nonoverlapping(src as *const u8, dst, size) };
                Ok(())
            },
            ElfSource::User { vmem, base, .. } => {
                let src: usize = base
                    .into_raw_value()
                    .checked_add(offset)
                    .ok_or_else(|| Error::new(ErrorCode::BadFile, "elf user offset overflow"))?;
                vmem.copy_from_user_unaligned(
                    VirtualAddress::new(dst as usize),
                    VirtualAddress::new(src),
                    size,
                )
            },
        }
    }

    /// Reads and returns the ELF file header.
    fn read_header(&self) -> Result<Elf32Fhdr, Error> {
        let mut hdr: MaybeUninit<Elf32Fhdr> = MaybeUninit::uninit();
        self.read_bytes(0, hdr.as_mut_ptr() as *mut u8, Elf32Fhdr::SIZE)?;
        // SAFETY: `Elf32Fhdr` is `repr(C)` and composed solely of integer fields with no invalid
        // bit patterns; the full `SIZE` bytes were just initialized.
        Ok(unsafe { hdr.assume_init() })
    }

    /// Reads and returns the program header at `index`, given the program header table file offset.
    fn read_phdr(&self, index: usize, e_phoff: usize) -> Result<Elf32Phdr, Error> {
        let entry_size: usize = core::mem::size_of::<Elf32Phdr>();
        let offset: usize = index
            .checked_mul(entry_size)
            .and_then(|o| o.checked_add(e_phoff))
            .ok_or_else(|| Error::new(ErrorCode::BadFile, "program header offset overflow"))?;
        let mut phdr: MaybeUninit<Elf32Phdr> = MaybeUninit::uninit();
        self.read_bytes(offset, phdr.as_mut_ptr() as *mut u8, entry_size)?;
        // SAFETY: `Elf32Phdr` is `repr(C)` and composed solely of `u32` fields with no invalid bit
        // patterns; the full entry was just initialized.
        Ok(unsafe { phdr.assume_init() })
    }

    ///
    /// # Description
    ///
    /// Copies `size` bytes of segment data at file `offset` into the destination address space at
    /// `dst`.
    ///
    /// On the dry-run validation pass this is a no-op for the [`ElfSource::User`] case, because the
    /// destination pages are not mapped until the real pass; the boot blob path still validates via
    /// its existing dry-run handling.
    ///
    fn copy_segment(
        &self,
        offset: usize,
        dst_vmem: &mut Vmem,
        dst: PageAligned<VirtualAddress>,
        size: usize,
        dry_run: bool,
    ) -> Result<(), Error> {
        self.check_bounds(offset, size)?;
        match self {
            ElfSource::Blob { base } => {
                let src: usize = base.checked_add(offset).ok_or_else(|| {
                    Error::new(ErrorCode::BadFile, "elf blob segment offset overflow")
                })?;
                dst_vmem.copy_to_user_unaligned_unchecked(
                    dst.into_inner(),
                    VirtualAddress::from_raw_value(src),
                    size,
                    dry_run,
                )
            },
            ElfSource::User { vmem, base, .. } => {
                // The destination pages are only mapped on the real pass.
                if dry_run {
                    return Ok(());
                }
                let src: usize = base.into_raw_value().checked_add(offset).ok_or_else(|| {
                    Error::new(ErrorCode::BadFile, "elf user segment offset overflow")
                })?;
                Vmem::copy_user_to_user(
                    vmem,
                    VirtualAddress::new(src),
                    dst_vmem,
                    dst.into_inner(),
                    size,
                )
            },
        }
    }
}

///
/// # Description
///
/// Loads an ELF32 binary into a target virtual memory space, reading the image bytes from the
/// given [`ElfSource`].
///
/// # Parameters
///
/// - `mm`: Virtual memory manager.
/// - `vmem`: Target virtual memory space.
/// - `source`: Where the ELF image bytes are read from.
/// - `dry_run`: When `true`, validate without allocating or copying (only meaningful for the
///   contiguous blob source).
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
    source: ElfSource,
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
    let header: Elf32Fhdr = source.read_header()?;
    if let Err(reason) = header.validate() {
        error!("{reason}");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    let e_phoff: usize = header.e_phoff as usize;
    let e_phnum: usize = header.e_phnum as usize;
    const MAX_PROGRAM_HEADERS: usize = 256;
    if e_phnum > MAX_PROGRAM_HEADERS {
        let reason: &str = "too many program headers";
        error!("{reason} (e_phnum={e_phnum})");
        return Err(Error::new(ErrorCode::BadFile, reason));
    }
    // Only ET_EXEC and ET_DYN binaries are supported.
    if header.e_type != ET_EXEC && header.e_type != ET_DYN {
        let reason: &str = "unsupported ELF type";
        error!("{reason} (e_type={:#x})", header.e_type);
        return Err(Error::new(ErrorCode::BadFile, reason));
    }

    // Compute load base for PIE (ET_DYN) binaries. If the lowest PT_LOAD virtual address is
    // below USER_BASE, offset all segment addresses so they land in user space.
    let load_base: usize = if header.e_type == ET_DYN {
        let user_base: usize = USER_BASE_RAW;
        let mut lowest_vaddr: usize = usize::MAX;
        for i in 0..e_phnum {
            let phdr: Elf32Phdr = source.read_phdr(i, e_phoff)?;
            if phdr.p_type == PT_LOAD {
                lowest_vaddr = lowest_vaddr.min(phdr.p_vaddr as usize);
            }
        }
        let lowest_vaddr: usize = if lowest_vaddr == usize::MAX {
            0
        } else {
            lowest_vaddr
        };
        if lowest_vaddr < user_base {
            user_base.saturating_sub(lowest_vaddr)
        } else {
            0
        }
    } else {
        0
    };

    let entry_raw: usize = (header.e_entry as usize)
        .checked_add(load_base)
        .ok_or_else(|| {
            let reason: &str = "entry address overflow";
            error!("{reason} (e_entry={:#x}, load_base={:#x})", header.e_entry, load_base);
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
    for phdr_index in 0..e_phnum {
        let phdr: Elf32Phdr = source.read_phdr(phdr_index, e_phoff)?;
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

        // The per-page copy below always writes a segment's file bytes starting at the page base
        // (`virt_addr_base`). This is only correct when the segment's load address coincides with
        // that base; a PT_LOAD segment that begins partway into a page would have its bytes placed
        // `adjusted_vaddr - virt_addr_base` bytes too low. Because `execv()` loads untrusted ELF
        // images, reject such unaligned segments instead of silently misplacing their data.
        if adjusted_vaddr != virt_addr_base {
            let reason: &str = "unaligned PT_LOAD segment";
            error!(
                "{reason} (adjusted_vaddr={adjusted_vaddr:#x}, virt_addr_base={virt_addr_base:#x})"
            );
            return Err(Error::new(ErrorCode::BadFile, reason));
        }

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

        // File-offset range of this segment's on-disk data. Segment bytes are read from the image
        // source relative to these offsets, independently of where (or how contiguously) the
        // source is stored.
        let file_off_base: usize = phdr.p_offset as usize;
        // `p_offset` and `p_filesz` are attacker-controlled in the `execv()` path; a crafted ELF
        // could overflow this sum, so it is computed with checked arithmetic.
        let file_off_end: usize = file_off_base
            .checked_add(phdr.p_filesz as usize)
            .ok_or_else(|| {
                let reason: &str = "segment file offset range overflow";
                error!("{reason} (p_offset={:#x}, p_filesz={:#x})", phdr.p_offset, phdr.p_filesz);
                Error::new(ErrorCode::BadFile, reason)
            })?;

        // Load segment page by page.
        debug!(
            "loading segment (virt_addr_base={:#x}, virt_addr_end={:#x}, file_off_base={:#x}, \
             file_off_end={:#x}, access={:?})",
            virt_addr_base, virt_addr_end, file_off_base, file_off_end, access
        );

        let mut uframe_buf: Vec<UserFrame> = crate::mm::try_vec_with_capacity(1)?;
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

                    // File offset of the source-backed data for this page.
                    let page_offset_in_segment: usize = vaddr.into_raw_value() - virt_addr_base;
                    let page_file_off: usize =
                        match file_off_base.checked_add(page_offset_in_segment) {
                            Some(off) => off,
                            None => {
                                let reason: &str = "invalid segment file offset";
                                error!("{reason}");
                                return Err(Error::new(ErrorCode::BadFile, reason));
                            },
                        };
                    // One-past-the-end if the full page were backed by segment data.
                    let page_file_off_end: usize = match page_file_off.checked_add(mem::PAGE_SIZE) {
                        Some(end) => end,
                        None => {
                            let reason: &str = "invalid segment file offset range";
                            error!("{reason}");
                            return Err(Error::new(ErrorCode::BadFile, reason));
                        },
                    };

                    // Page is entirely beyond segment data (pure BSS) — must be zeroed.
                    let page_lies_in_bss: bool = page_file_off >= file_off_end;
                    // Page straddles the segment-data/BSS boundary — trailing bytes must
                    // be zeroed.
                    let page_is_partially_covered: bool =
                        page_file_off < file_off_end && page_file_off_end > file_off_end;
                    mm.alloc_upages(
                        vmem,
                        vaddr,
                        access,
                        page_lies_in_bss || page_is_partially_covered,
                        1,
                        &mut uframe_buf,
                    )?;
                }
            }

            // Update last address.
            if vaddr.into_raw_value() + mem::PAGE_SIZE > last_address {
                last_address = vaddr.into_raw_value() + mem::PAGE_SIZE;
            }

            // `file_off_base` is attacker-controlled in the `execv()` path; guard the running
            // offset against overflow. The subtraction is safe because the loop starts at
            // `virt_addr_base` and never produces a `vaddr` below it.
            let file_off: usize = file_off_base
                .checked_add(vaddr.into_raw_value() - virt_addr_base)
                .ok_or_else(|| {
                    let reason: &str = "segment file offset overflow";
                    error!("{reason} (file_off_base={file_off_base:#x})");
                    Error::new(ErrorCode::BadFile, reason)
                })?;

            // Load segment only if it is within bounds.
            if file_off < file_off_end {
                let size: usize = min(mem::PAGE_SIZE, file_off_end - file_off);

                // Load segment only if it has a non-zero size.
                if size > 0 {
                    source.copy_segment(file_off, vmem, vaddr, size, dry_run)?;
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
    let source: ElfSource = ElfSource::Blob {
        base: elf as *const Elf32Fhdr as usize,
    };
    if cfg!(feature = "nightly-performance-optimizations") {
        do_elf32_load(mm, vmem, source, false)
    } else {
        // Two-pass: first validate with a dry run, then load. The dry run reads from a contiguous,
        // kernel-addressable blob, so it can validate the full layout before any mapping.
        do_elf32_load(mm, vmem, source, true)?;
        do_elf32_load(mm, vmem, source, false)
    }
}

///
/// # Description
///
/// Loads an ELF32 binary into `dst_vmem`, reading the image directly from the user address space
/// `src_vmem` at virtual address `base` (spanning `len` bytes).
///
/// This is the `execv()` loader. Unlike [`elf32_load`], the image is not a contiguous kernel blob:
/// it lives in the calling process's address space (placed there by `mmap`) and may be physically
/// non-contiguous, so the loader streams each segment page-by-page from the source address space
/// into freshly allocated pages of the destination. Because the destination pages are only mapped
/// on the loading pass, a separate validating dry run is not performed; the header and program
/// headers are still validated before any mapping occurs.
///
/// # Parameters
///
/// - `mm`: Virtual memory manager.
/// - `dst_vmem`: Destination address space for the new image.
/// - `src_vmem`: Source address space that contains the ELF image.
/// - `base`: Virtual address of the ELF image within `src_vmem`.
/// - `len`: Length of the ELF image in bytes.
///
/// # Returns
///
/// Upon successful completion, the entry point of the ELF32 binary and the address past the last
/// loaded segment are returned. Otherwise, an error is returned and `dst_vmem` may be left in an
/// inconsistent state.
///
pub fn elf32_load_from_user(
    mm: &mut VirtMemoryManager,
    dst_vmem: &mut Vmem,
    src_vmem: &Vmem,
    base: VirtualAddress,
    len: usize,
) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> {
    let source: ElfSource = ElfSource::User {
        vmem: src_vmem,
        base,
        len,
    };
    do_elf32_load(mm, dst_vmem, source, false)
}
