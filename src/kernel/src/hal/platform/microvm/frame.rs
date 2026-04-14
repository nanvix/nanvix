// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Microvm-side frame allocator implementation.
//!
//! Backed by a single [`SparseBitmap`] whose chunks cover every physical
//! frame range the allocator tracks. The typical configuration is one
//! dense chunk at offset 0 covering the identity-mapped physical address
//! range. Foreign-address registration is supported via
//! [`SparseBitmap::set_or_add_chunk`] for callers that hand out frames
//! outside the dense range.

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
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
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

/// Static byte array backing the dense identity-range bitmap. Sized for
/// `MEMORY_SIZE` worth of frames (one bit per frame).
static mut FRAME_ALLOCATOR_STORAGE: [u8; config::kernel::MEMORY_SIZE
    / (mem::FRAME_SIZE * u8::BITS as usize)] =
    [0; config::kernel::MEMORY_SIZE / (mem::FRAME_SIZE * u8::BITS as usize)];

//==================================================================================================
// Inner
//==================================================================================================

/// Microvm-side frame allocator inner state. Owned directly by
/// [`crate::mm::phys::FrameAllocator`].
#[derive(Debug)]
pub struct Inner {
    bitmap: SparseBitmap,
    /// Cached capacity of the dense identity-range chunk at offset 0,
    /// in frames. Used to fast-classify frame addresses as "in the
    /// dense range" without walking chunks.
    #[allow(dead_code)]
    dense_range_frames: usize,
}

impl Inner {
    /// Constructs the allocator from the BSS-backed storage. Must be
    /// called exactly once during boot.
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

    pub fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let raw_index: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?}");
                return Err(error);
            },
        };
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(raw_index) {
            Some(frame_number) => frame_number,
            None => {
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => Ok(frame_address),
            Err(error) => {
                error!("{error:?}");
                Err(error)
            },
        }
    }

    pub fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();
        if let Err(error) = self.bitmap.clear(frame_number) {
            error!("{error:?} (frame={frame:?})");
            return Err(error);
        }
        Ok(())
    }

    pub fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
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
        let start_frame_number: usize = region.start().into_frame_number().into_raw_value();
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
