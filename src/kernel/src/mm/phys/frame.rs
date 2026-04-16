// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::{
        Bitmap,
        RawArray,
    },
    hal::mem::{
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
};
use ::alloc::vec;
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
use ::config::constants;
use ::sparse_bitmap::SparseBitmap;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Address,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Frame allocator.
///
#[derive(Debug)]
pub struct FrameAllocator {
    /// A sparse bitmap that keeps track of free/used frames.
    bitmap: SparseBitmap,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl FrameAllocator {
    ///
    /// # Description
    ///
    /// Instantiates a frame allocator.
    ///
    /// # Parameters
    ///
    /// - `bitmap`: A sparse bitmap to keep track of free/used frames.
    ///
    pub fn new(bitmap: SparseBitmap) -> Self {
        let frame_allocator: FrameAllocator = Self { bitmap };

        info!(
            "frame allocator capacity: {} frames, {} MB",
            frame_allocator.bitmap.capacity(),
            (frame_allocator.bitmap.capacity() * mem::FRAME_SIZE) / constants::MEGABYTE
        );

        frame_allocator
    }

    ///
    /// # Description
    ///
    /// Instantiates a frame allocator from raw byte storage.
    ///
    /// # Parameters
    ///
    /// - `storage`: A raw byte array to use as backing storage for the bitmap.
    ///
    /// # Returns
    ///
    /// Upon success, the constructed frame allocator is returned. Upon failure, an error is
    /// returned instead.
    ///
    pub fn from_raw_storage(storage: RawArray<u8>) -> Result<Self, Error> {
        let bitmap: Bitmap = Bitmap::from_raw_array(storage)?;
        let sparse: SparseBitmap = SparseBitmap::new(vec![(0, bitmap)])?;
        Ok(Self::new(sparse))
    }

    ///
    /// # Description
    ///
    /// Allocates a frame.
    ///
    /// # Returns
    ///
    /// Upon success, the index of the allocated frame is returned. Upon failure, an error is
    /// returned instead.
    ///
    pub fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let frame_number: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?}");
                return Err(error);
            },
        };
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                let reason: &str = "frame number is out of bounds";
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Attempt to convert the frame number to a frame address.
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => Ok(frame_address),
            Err(error) => {
                error!("{error:?}");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Frees a frame that was previous allocated.
    ///
    /// # Parameters
    ///
    /// - `frame`: Index of the frame to free.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        let frame_number: usize = frame.into_frame_number().into_raw_value();
        match self.bitmap.clear(frame_number) {
            Ok(()) => Ok(()),
            Err(error) => {
                error!("{error:?} (frame={frame:?})");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Books a frame that was previously allocated.
    ///
    /// # Parameters
    ///
    /// - `phys_addr`: Physical address of the frame to book.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
        match self.bitmap.set(frame_number) {
            Ok(()) => Ok(()),
            Err(error) => {
                error!("{error:?} (phys_addr={phys_addr:?})");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Allocates all frames in the range `[start, end]`.
    ///
    /// # Parameters
    ///
    /// - `start`: Start page frame address.
    /// - `end`: End page frame address.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        let start_frame_number: usize = region.start().into_frame_number().into_raw_value();
        let end_frame_number: usize = start_frame_number + region.size() / mem::FRAME_SIZE - 1;

        // Check if all frames in the range are free.
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

        // Book all frames in the range.
        for index in start_frame_number..=end_frame_number {
            if let Err(error) = self.bitmap.set(index) {
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
        }

        Ok(())
    }
}
