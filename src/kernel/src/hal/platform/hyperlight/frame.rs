// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Hyperlight-side frame allocator implementation.
//!
//! Bookkeeping is the same shape as the microvm allocator: a single
//! [`SparseBitmap`] with a dense chunk at offset 0 covering identity-
//! mapped RAM, plus chunks added on demand for foreign-address frames
//! (scratch). The difference is `alloc()` — on Hyperlight the dense
//! range is EPT-read-only, so allocations come from the scratch bump
//! cursor rather than the dense bitmap, and are tracked via
//! [`SparseBitmap::add_chunk`] + [`SparseBitmap::set`].

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
// Constants
//==================================================================================================

/// Default chunk size (in bits) for chunks added on demand as scratch
/// frames are booked. Sized to cover the entire scratch region in a
/// single chunk (~16 MB / 4 KB = 4096 frames).
const FOREIGN_CHUNK_BITS: usize = 4096;

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
    #[allow(dead_code)]
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

        Ok(Self {
            bitmap: SparseBitmap::new(::alloc::vec![(0, dense)])?,
            dense_range_frames,
        })
    }

    /// On Hyperlight, the frame comes from the scratch bump cursor
    /// (the dense identity range is EPT-read-only). The result is
    /// booked in the sparse bitmap so a later [`Self::free`] can
    /// release it.
    pub fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let frame: FrameAddress = crate::mm::Vmem::alloc_scratch_frame()?;
        let index: usize = frame.into_raw_value() / mem::PAGE_SIZE;
        if self.bitmap.find_chunk(index).is_none() {
            let chunk_offset: usize = (index / FOREIGN_CHUNK_BITS) * FOREIGN_CHUNK_BITS;
            self.bitmap
                .add_chunk(chunk_offset, Bitmap::new(FOREIGN_CHUNK_BITS)?)?;
        }
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

    pub fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        let frame_number: usize = phys_addr.into_raw_value() / mem::PAGE_SIZE;
        match self.bitmap.set(frame_number) {
            Ok(()) => Ok(()),
            Err(error) => {
                trace!("{error:?} (phys_addr={phys_addr:?})");
                Err(error)
            },
        }
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
