// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::FrameAddress,
    mm::phys::frame::FrameAllocator,
};
use ::alloc::{
    rc::Rc,
    vec::Vec,
};
use ::core::cell::RefCell;
use ::sys::error::Error;

//==================================================================================================
// User Frame Pool Inner
//==================================================================================================

///
/// # Description
///
/// A structure that describes a pool of user frames.
///
#[derive(Debug)]
struct UpoolInner {
    /// Underlying frame allocator.
    frame_allocator: FrameAllocator,
}

impl UpoolInner {
    ///
    /// # Description
    ///
    /// Instantiates a user frame pool.
    ///
    /// # Parameters
    ///
    /// - `frame_allocator`: Underlying frame allocator.
    ///
    /// # Returns
    ///
    /// A user frame pool.
    ///
    fn new(frame_allocator: FrameAllocator) -> Self {
        Self { frame_allocator }
    }

    ///
    /// # Description
    ///
    /// Allocates a frame from the user frame pool.
    ///
    /// # Returns
    ///
    /// On success, the physical address of the allocated frame is returned. On failure, an error is
    /// returned.
    ///
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        self.frame_allocator.alloc()
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of frames from the user frame pool.
    ///
    /// # Parameters
    ///
    /// - `size`: Number of frames to allocate.
    ///
    /// # Returns
    ///
    /// Upon success, a vector of physical addresses is returned. Upon failure, an error is returned
    /// instead.
    fn alloc_many(&mut self, size: usize) -> Result<Vec<FrameAddress>, Error> {
        let mut pages: Vec<FrameAddress> = Vec::new();

        for _ in 0..size {
            pages.push(self.frame_allocator.alloc()?);
        }

        Ok(pages)
    }

    ///
    /// # Description
    ///
    /// Frees a frame that was previously allocated from the user frame pool.
    ///
    /// # Parameters
    ///
    /// - `frame_addr`: Physical address of the frame to be freed.
    ///
    /// # Returns
    ///
    /// On success, `Ok(())` is returned. On failure, an error is returned.
    ///
    fn free(&mut self, page_addr: FrameAddress) -> Result<(), Error> {
        self.frame_allocator.free(page_addr)
    }
}

//==================================================================================================
// User Frame
//==================================================================================================

///
/// # Description
///
/// A type that represents a user frame.
///
#[derive(Debug)]
pub struct UserFrame {
    /// Frame address.
    addr: FrameAddress,
}

impl UserFrame {
    ///
    /// # Description
    ///
    /// Instantiates a user frame.
    ///
    /// # Parameters
    ///
    /// - `addr`: Frame address.
    /// - `upool`: Back reference to the user frame pool.
    ///
    /// # Returns
    ///
    /// A user frame.
    ///
    pub fn new(addr: FrameAddress) -> Self {
        Self { addr }
    }

    ///
    /// # Description
    ///
    /// Returns the physical address of the target user frame.
    ///
    /// # Returns
    ///
    /// The physical address of the target user frame.
    ///
    pub fn address(&self) -> FrameAddress {
        self.addr
    }
}

//==================================================================================================
// User Frame Pool
//==================================================================================================

///
/// # Description
///
/// A structure that describes a pool of user frames.
///
#[derive(Debug)]
pub struct Upool {
    /// Inner data structure.
    inner: Rc<RefCell<UpoolInner>>,
}

impl Upool {
    ///
    /// # Description
    ///
    /// Instantiates a user frame pool.
    ///
    /// # Parameters
    ///
    /// - `frame_allocator`: Underlying frame allocator.
    ///
    /// # Returns
    ///
    /// A user frame pool.
    ///
    pub fn new(frame_allocator: FrameAllocator) -> Self {
        Self {
            inner: Rc::new(RefCell::new(UpoolInner::new(frame_allocator))),
        }
    }

    ///
    /// # Description
    ///
    /// Allocates a frame from the user frame pool.
    ///
    /// # Returns
    ///
    /// On success, the physical address of the allocated frame is returned, with
    /// read-only permissions and all bytes set to zero. On failure, an error is
    /// returned instead.
    ///
    pub fn alloc(&mut self) -> Result<UserFrame, Error> {
        let addr: FrameAddress = self.inner.borrow_mut().alloc()?;
        let uframe: UserFrame = UserFrame::new(addr);
        Ok(uframe)
    }

    pub fn alloc_many(&mut self, nframes: usize) -> Result<Vec<UserFrame>, Error> {
        trace!("nframes={nframes:?}");

        // Attempt to allocate pages.
        let mut uframes: Vec<FrameAddress> = self.inner.borrow_mut().alloc_many(nframes)?;

        // Create a vector of user pages.
        let mut upages: Vec<UserFrame> = Vec::new();
        while let Some(page) = uframes.pop() {
            let upage: UserFrame = UserFrame::new(page);
            upages.push(upage);
        }

        Ok(upages)
    }

    ///
    /// # Description
    ///
    /// Frees a frame that was previously allocated from the user frame pool.
    ///
    /// # Parameters
    ///
    /// - `uframe`: User frame to be freed.
    ///
    /// # Returns
    ///
    /// On success, empty is returned. On failure, an error is returned instead.
    ///
    pub fn free(&mut self, uframe: UserFrame) -> Result<(), Error> {
        self.inner.borrow_mut().free(uframe.address())
    }
}
