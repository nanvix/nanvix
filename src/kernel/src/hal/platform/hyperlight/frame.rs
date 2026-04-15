// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Hyperlight-side frame allocator implementation.
//!
//! Two-chunk [`SparseBitmap`] pre-provisioned at init: a dense chunk at
//! offset 0 covering identity-mapped RAM, plus a scratch chunk at
//! `scratch_base / PAGE_SIZE` covering the full scratch region. Both
//! are declared up front via `SparseBitmap::new` — the bitmap shape is
//! fixed after construction. `alloc()` pulls from the scratch bump
//! cursor (the dense range is EPT-read-only) and records the result
//! with a plain `bitmap.set`.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::{
        Bitmap,
        RawArray,
        SparseBitmap,
    },
    hal::mem::{
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
};
use ::arch::mem;
use ::config::constants;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Address,
};

//==================================================================================================
// Backing storage
//==================================================================================================

/// Static byte array backing the dense identity-range bitmap.
static mut FRAME_ALLOCATOR_STORAGE: [u8; config::kernel::MEMORY_SIZE
    / (mem::FRAME_SIZE * u8::BITS as usize)] =
    [0; config::kernel::MEMORY_SIZE / (mem::FRAME_SIZE * u8::BITS as usize)];

//==================================================================================================
// Inner
//==================================================================================================

/// Hyperlight-side frame allocator inner state. Owned directly by
/// [`crate::mm::phys::FrameAllocator`].
#[derive(Debug)]
pub struct Inner {
    bitmap: SparseBitmap,
    /// Cached capacity of the dense identity-range chunk at offset 0.
    /// Used to fast-classify frame addresses as inside or outside the
    /// dense range.
    dense_range_frames: usize,
}

impl Inner {
    pub fn new() -> Result<Self, Error> {
        let storage: RawArray<u8> = unsafe {
            let (ptr, len): (*mut u8, usize) =
                (FRAME_ALLOCATOR_STORAGE.as_mut_ptr(), FRAME_ALLOCATOR_STORAGE.len());
            RawArray::from_raw_parts(ptr, len)?
        };
        let dense: Bitmap = Bitmap::from_raw_array(storage)?;
        let dense_range_frames: usize = dense.number_of_bits();

        info!(
            "frame allocator capacity: {} frames, {} MB",
            dense_range_frames,
            dense_range_frames * mem::FRAME_SIZE / constants::MEGABYTE
        );

        // Scratch chunk: pre-provisioned at init so `alloc()` can just
        // `bitmap.set(frame_index)` against the scratch bump cursor's
        // result without ever growing the bitmap.
        let (scratch_offset, scratch_frames) = crate::mm::Vmem::scratch_range_in_frames();
        let scratch_bitmap: Bitmap = Bitmap::new(scratch_frames)?;

        Ok(Self {
            bitmap: SparseBitmap::new(::alloc::vec![
                (0, dense),
                (scratch_offset, scratch_bitmap),
            ])?,
            dense_range_frames,
        })
    }

    /// On Hyperlight, the frame comes from the scratch bump cursor
    /// (the dense identity range is EPT-read-only). The resulting GPA
    /// is booked in the scratch chunk of the sparse bitmap — the chunk
    /// was pre-provisioned by [`Self::new`], so the `bitmap.set`
    /// never needs to grow state.
    pub fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let frame: FrameAddress = crate::mm::Vmem::alloc_scratch_frame()?;
        let index: usize = frame.into_raw_value() / mem::PAGE_SIZE;
        self.bitmap.set(index)?;
        Ok(frame)
    }

    /// Frees a frame. Untracked addresses are silently ignored —
    /// scratch is wiped on every snapshot restore so any leak is
    /// harmless.
    pub fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_raw_value() / mem::PAGE_SIZE;
        match self.bitmap.clear(frame_number) {
            Ok(()) => Ok(()),
            Err(error) => {
                trace!("free(): {error:?} (frame={frame:?})");
                Ok(())
            },
        }
    }

    /// Books a frame. Only the dense identity-range chunk is eligible:
    /// the scratch chunk is populated exclusively by [`Self::alloc`] as
    /// the bump cursor hands GPAs out, so pre-booking scratch addresses
    /// (as `book_mmio_regions` does when it walks the HL "SCRATCHIO"
    /// region) would mark every scratch frame allocated and starve
    /// later `alloc()` calls. Addresses outside the dense range return
    /// `InvalidArgument` quietly — the MMIO booker already tolerates
    /// that for frames that fall outside its tracked range.
    pub fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        let frame_number: usize = phys_addr.into_raw_value() / mem::PAGE_SIZE;
        if frame_number >= self.dense_range_frames {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "frame lies outside the dense identity range",
            ));
        }
        self.bitmap.set(frame_number)
    }

    pub fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        let start_frame_number: usize = region.start().into_raw_value() / mem::PAGE_SIZE;
        let end_frame_number: usize = start_frame_number + region.size() / mem::FRAME_SIZE - 1;

        for index in start_frame_number..=end_frame_number {
            match self.bitmap.test(index) {
                Ok(false) => continue,
                Ok(true) => {
                    let conflicting_addr: usize = index * mem::FRAME_SIZE;
                    let region_start: usize = region.start().into_raw_value();
                    let region_end: usize = region_start.saturating_add(region.size());
                    let reason: &str = "frame is already allocated";
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason, conflicting_addr, region_start, region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => return Err(err),
            }
        }

        for index in start_frame_number..=end_frame_number {
            if let Err(error) = self.bitmap.set(index) {
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
        }

        Ok(())
    }
}
