// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # ELF64 Loading Support
//!
//! This module provides ELF64 structures and loading functions for 64-bit binaries.
//!

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        AccessPermission,
        PageAligned,
        VirtualAddress,
    },
    mm::{
        VirtMemoryManager,
        Vmem,
    },
};
use ::arch::{
    mem,
    mem::PAGE_ALIGNMENT,
};
use ::core::cmp::{
    max,
    min,
};
use ::sys::{
    config,
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        Address,
        Alignment,
    },
};

use ::elf::{
    elf32::{
        PF_R,
        PF_W,
        PF_X,
        PT_LOAD,
    },
    elf64::Elf64Phdr,
};

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use ::elf::elf64::Elf64Fhdr;

//==================================================================================================
// Functions
//==================================================================================================

fn do_elf64_load(
    mm: &mut VirtMemoryManager,
    vmem: &mut Vmem,
    elf: &Elf64Fhdr,
    dry_run: bool,
) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> {
    trace!("dry_run={}", dry_run);

    let mut last_address: usize = 0;

    if !elf.is_valid() {
        return Err(Error::new(ErrorCode::BadFile, "invalid elf file"));
    }

    let entry: VirtualAddress = VirtualAddress::new(elf.e_entry as usize);

    // Check if entry point does not match what we expect.
    if entry < config::memory_layout::USER_BASE {
        let reason: &str = "invalid binary entry point";
        error!("do_elf64_load: {} (entry={:?})", reason, entry);
        return Err(Error::new(ErrorCode::BadFile, "invalid entry point"));
    }

    let phdr_base: *const Elf64Phdr = unsafe {
        (elf as *const Elf64Fhdr as *const u8).offset(elf.e_phoff as isize) as *const Elf64Phdr
    };
    let phdrs: &[Elf64Phdr] =
        unsafe { core::slice::from_raw_parts(phdr_base, elf.e_phnum as usize) };

    // Load segments.
    for phdr in phdrs {
        if phdr.p_type != PT_LOAD {
            continue;
        }

        // Check if the segment is not valid.
        if phdr.p_filesz > phdr.p_memsz {
            return Err(Error::new(ErrorCode::BadFile, "corrupted elf file"));
        }

        let align: Alignment = (phdr.p_align as usize)
            .try_into()
            .map_err(|_| Error::new(ErrorCode::BadFile, "invalid alignment value in elf file"))?;
        let virt_addr_base: usize = ::sys::mm::align_down(phdr.p_vaddr as usize, align);

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
        let virt_addr_range_end: usize = virt_addr_base.checked_add(size).ok_or_else(|| {
            let reason: &str = "virtual address overflow in elf segment";
            error!("do_elf64_load(): {reason} (virt_addr_base={virt_addr_base:#x}, size={size})");
            Error::new(ErrorCode::BadFile, reason)
        })?;
        let virt_addr_end: usize = ::sys::mm::align_up(virt_addr_range_end, PAGE_ALIGNMENT)
            .ok_or_else(|| {
                let reason: &str = "align_up overflow";
                error!("do_elf64_load(): {reason} (virt_addr_range_end={virt_addr_range_end:#x})");
                Error::new(ErrorCode::BadFile, reason)
            })?;

        let phys_addr_base: usize = unsafe {
            (elf as *const Elf64Fhdr as *const u8).offset(phdr.p_offset as isize) as usize
        };

        let phys_addr_end: usize = phys_addr_base + phdr.p_filesz as usize;

        // Load segment page by page.
        debug!(
            "do_elf64_load(): loading segment (virt_addr_base={:#x}, virt_addr_end={:#x}, \
             phys_addr_base={:#x}, phys_addr_end={:#x}, access={:?})",
            virt_addr_base, virt_addr_end, phys_addr_base, phys_addr_end, access
        );

        for vaddr in (virt_addr_base..virt_addr_end).step_by(mem::PAGE_SIZE) {
            let vaddr: VirtualAddress = VirtualAddress::new(vaddr);

            // Check if address lies in user space.
            if vaddr < config::memory_layout::USER_BASE {
                let reason: &str = "invalid load address";
                error!("do_elf64_load: {}", reason);
                return Err(Error::new(ErrorCode::BadFile, reason));
            }

            let vaddr: PageAligned<VirtualAddress> = PageAligned::from_address(vaddr)?;

            // Check if we should perform the allocation.
            if !dry_run {
                // Allocate page.
                mm.alloc_upages(vmem, vaddr, 1, access, true)?;
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
    }

    let aligned_last: VirtualAddress = VirtualAddress::new(last_address)
        .align_up(PAGE_ALIGNMENT)
        .ok_or_else(|| {
        let reason: &str = "align_up overflow";
        error!("do_elf64_load(): {reason} (last_address={last_address:#x})");
        Error::new(ErrorCode::BadFile, reason)
    })?;

    Ok((entry, PageAligned::from_address(aligned_last)?))
}

pub fn elf64_load(
    mm: &mut VirtMemoryManager,
    vmem: &mut Vmem,
    elf: &Elf64Fhdr,
) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error> {
    do_elf64_load(mm, vmem, elf, true)?;
    do_elf64_load(mm, vmem, elf, false)
}
