// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mm::phys::{
    kpool::{
        KernelFrame,
        Kpool,
    },
    upool::{
        Upool,
        UserFrame,
    },
};
use ::alloc::vec::Vec;
use ::sys::error::Error;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub struct PhysMemoryManager {
    kpool: Kpool,
    upool: Upool,
}

impl PhysMemoryManager {
    pub fn new(kpool: Kpool, upool: Upool) -> Self {
        PhysMemoryManager { kpool, upool }
    }

    pub fn alloc_user_frame(&mut self) -> Result<UserFrame, Error> {
        self.upool.alloc()
    }

    pub fn alloc_many_user_frames(&mut self, nframes: usize) -> Result<Vec<UserFrame>, Error> {
        self.upool.alloc_many(nframes)
    }

    ///
    /// # Description
    ///
    /// Allocates a kernel frame.
    ///
    /// # Parameters
    ///
    /// - `clear`: Clear frame?
    ///
    /// # Return Values
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc_kernel_frame(&mut self, clear: bool) -> Result<KernelFrame, Error> {
        self.kpool.alloc(clear)
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel frames.
    ///
    /// # Parameters
    ///
    /// - `clear`: Clear frames?
    /// - `count`: Number of frames to allocate.
    ///
    /// # Return Values
    ///
    /// Upon success, a vector of kernel frames is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn alloc_many_kernel_frames(
        &mut self,
        clear: bool,
        count: usize,
    ) -> Result<Vec<KernelFrame>, Error> {
        self.kpool.alloc_many(clear, count)
    }
}
