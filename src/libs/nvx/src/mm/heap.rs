// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

// The following imports are used only when any logging feature is enabled.
#[allow(unused_imports)]
use crate::logging::{
    LogLevel,
    Logger,
};
#[allow(unused_imports)]
use ::core::fmt::Write;

use crate::mm::PAGE_ALIGNMENT;
use ::sys::{
    arch::mem,
    error::{
        Error,
        ErrorCode,
    },
    kcall,
    mm::{
        self,
        AccessPermission,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
//  Structures
//==================================================================================================

pub struct Heap {
    pid: ProcessIdentifier,
    base: VirtualAddress,
    size: usize,
    capacity: usize,
}

impl Heap {
    pub fn new(
        pid: ProcessIdentifier,
        base: VirtualAddress,
        size: usize,
        capacity: usize,
    ) -> Result<Self, Error> {
        #[cfg(feature = "trace")]
        let _ = writeln!(
            &mut Logger::get(module_path!(), LogLevel::Trace),
            "new(): base={:X?}, size={:X?}, capacity={:X?}",
            base,
            size,
            capacity
        );

        // Check if base address is page-aligned.
        if !base.is_aligned(PAGE_ALIGNMENT) {
            #[cfg(feature = "error")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Error),
                "new(): unaligned base address {:X?}",
                base
            );
            return Err(Error::new(ErrorCode::BadAddress, "unaligned base address"));
        }

        // Check if size is zero.
        if size == 0 {
            #[cfg(feature = "error")]
            let _ = writeln!(&mut Logger::get(module_path!(), LogLevel::Error), "new(): zero size");
            return Err(Error::new(ErrorCode::BadAddress, "zero size"));
        }

        // Check if capacity is zero.
        if capacity == 0 {
            #[cfg(feature = "error")]
            let _ =
                writeln!(&mut Logger::get(module_path!(), LogLevel::Error), "new(): zero capacity");
            return Err(Error::new(ErrorCode::BadAddress, "zero capacity"));
        }

        // Check if capacity is smaller than size.
        if capacity < size {
            #[cfg(feature = "error")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Error),
                "new(): capacity is too small"
            );
            return Err(Error::new(ErrorCode::BadAddress, "capacity is too small"));
        }

        // Map initial pages.
        let start: VirtualAddress = base;
        let end: VirtualAddress = VirtualAddress::from_raw_value(base.into_raw_value() + size);
        map_range(pid, start, end)?;

        Ok(Self {
            pid,
            base,
            size,
            capacity,
        })
    }

    pub fn base(&self) -> VirtualAddress {
        self.base
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn grow(&mut self, increment: usize) -> Result<(), Error> {
        #[cfg(feature = "trace")]
        let _ = writeln!(
            &mut Logger::get(module_path!(), LogLevel::Trace),
            "grow(): increment={:X?}",
            increment
        );

        // Check if increment is page-aligned.
        if !mm::is_aligned(increment, PAGE_ALIGNMENT) {
            #[cfg(feature = "error")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Error),
                "grow(): unaligned increment"
            );
            return Err(Error::new(ErrorCode::BadAddress, "unaligned increment"));
        }

        // Check if increment is zero.
        if increment == 0 {
            #[cfg(feature = "error")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Error),
                "grow(): zero increment"
            );
            return Err(Error::new(ErrorCode::BadAddress, "zero increment"));
        }

        // Check if increment would exceed capacity.
        if self.size + increment > self.capacity {
            #[cfg(feature = "error")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Error),
                "grow(): exceeds capacity"
            );
            return Err(Error::new(ErrorCode::BadAddress, "exceeds capacity"));
        }

        // Map pages.
        let end: VirtualAddress = self.base + self.size;
        let new_end: VirtualAddress = end + increment;
        map_range(self.pid, end, new_end)?;

        // Update metadata.
        self.size += increment;

        Ok(())
    }
}

/// Map pages in the range [start, end).
pub fn map_range(
    pid: ProcessIdentifier,
    start: VirtualAddress,
    end: VirtualAddress,
) -> Result<(), Error> {
    #[cfg(feature = "trace")]
    let _ = writeln!(
        &mut Logger::get(module_path!(), LogLevel::Trace),
        "map_range(): start={:X?}, end={:X?}",
        start,
        end
    );

    debug_assert!(start.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(end.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(start < end);

    // TODO: use iterator.
    let start: usize = start.into_raw_value();
    let end: usize = end.into_raw_value();
    for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
        debug_assert!(vaddr != end);

        // Attempt to map page.
        let vaddr: VirtualAddress = VirtualAddress::new(vaddr);
        if let Err(error) = kcall::mm::mmap(pid, vaddr, AccessPermission::RDWR) {
            // Failed to map page, attempt to rollback.

            #[cfg(feature = "error")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Error),
                "map_range(): failed to map page at {:X?}, rolling back (error={:?})",
                vaddr,
                error
            );

            // Attempt to unmap pages.
            if let Err(_error) = unmap_range(pid, VirtualAddress::new(start), vaddr) {
                // Failed to unmap range, warn.
                #[cfg(feature = "warn")]
                let _ = writeln!(
                    &mut Logger::get(module_path!(), LogLevel::Warn),
                    "map_range(): failed to unmap pages at {:X?}..{:X?} (error={:?})",
                    start,
                    vaddr,
                    _error
                );
            }

            return Err(error);
        }

        // NOTE: pages allocated with mmap() are always zeroed.
    }

    Ok(())
}

/// Unmap pages in the range [start, end).
pub fn unmap_range(
    pid: ProcessIdentifier,
    start: VirtualAddress,
    end: VirtualAddress,
) -> Result<(), Error> {
    let _ = writeln!(
        &mut Logger::get(module_path!(), LogLevel::Trace),
        "unmap_range(): start={:X?}, end={:X?}",
        start,
        end
    );

    debug_assert!(start.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(end.is_aligned(PAGE_ALIGNMENT));
    debug_assert!(start < end);

    let mut ret: Result<(), Error> = Ok(());
    let start: usize = start.into_raw_value();
    let end: usize = end.into_raw_value();
    for vaddr in (start..end).step_by(mem::PAGE_SIZE) {
        debug_assert!(vaddr != end);

        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(vaddr);

        if let Err(error) = kcall::mm::munmap(pid, vaddr) {
            #[cfg(feature = "error")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Error),
                "unmap_range(): failed to unmap page at {:X?}, skipping (error={:?})",
                vaddr,
                error
            );

            // Save error.
            ret = Err(error);
        }
    }

    ret
}
impl Drop for Heap {
    fn drop(&mut self) {
        #[cfg(feature = "trace")]
        let _ = writeln!(
            &mut Logger::get(module_path!(), LogLevel::Trace),
            "drop(): base={:X?}, size={:X?}, capacity={:X?}",
            self.base,
            self.size,
            self.capacity
        );

        // Unmap pages.
        if let Err(_error) = unmap_range(
            self.pid,
            self.base,
            VirtualAddress::from_raw_value(self.base.into_raw_value() + self.size),
        ) {
            #[cfg(feature = "warn")]
            let _ = writeln!(
                &mut Logger::get(module_path!(), LogLevel::Warn),
                "drop(): failed to unmap pages (error={:?})",
                _error
            );
        }
    }
}
