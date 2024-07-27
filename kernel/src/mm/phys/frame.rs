// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem::{
        self,
        paging::FrameNumber,
    },
    hal::mem::{
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
    klib::{
        self,
        bitmap::Bitmap,
        raw_array::RawArray,
    },
};
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
            frame_allocator.bitmap.number_of_bits() * mem::FRAME_SIZE / klib::MEGABYTE
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
        let frame_number: usize = self.bitmap.alloc()?;
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                return Err(Error::new(ErrorCode::OutOfMemory, "frame number is out of bounds"));
            },
        };

        // Attempt to convert the frame number to a frame address.
        FrameAddress::from_frame_number(frame_number)
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
        self.bitmap.clear(frame_number)
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
        self.bitmap.set(frame_number)?;
        Ok(())
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
            if let Err(e) = self.bitmap.set(index) {
                warn!("fail to book frame {}, but we should not", index);
                return Err(e);
            }
        }

        Ok(())
    }
}
