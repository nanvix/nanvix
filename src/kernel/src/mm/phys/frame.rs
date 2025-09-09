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
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
use ::config::constants;
use ::sys::error::{
    Error,
    ErrorCode,
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
    /// A bitmap that keeps track of free/used frames.
    bitmap: Bitmap,
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
    /// - `bitmap`: A bitmap to keeps track of free/used frames.
    ///
    pub fn new(bitmap: Bitmap) -> Self {
        let frame_allocator: FrameAllocator = Self { bitmap };

        info!(
            "frame allocator capacity: {} frames, {} MB",
            frame_allocator.bitmap.number_of_bits(),
            frame_allocator.bitmap.number_of_bits() * mem::FRAME_SIZE / constants::MEGABYTE
        );

        frame_allocator
    }

    pub fn from_raw_storage(storage: RawArray<u8>) -> Result<Self, Error> {
        Ok(Self::new(Bitmap::from_raw_array(storage)))
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
            Ok(frame_number) => frame_number,
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
                    return Err(Error::new(ErrorCode::OutOfMemory, "frame is already allocated"));
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
