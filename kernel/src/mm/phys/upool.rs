// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    error::Error,
    hal::mem::FrameAddress,
    mm::phys::frame::FrameAllocator,
};
use alloc::{
    rc::Rc,
    vec::Vec,
};
use core::{
    cell::RefCell,
    ops::{
        Deref,
        DerefMut,
    },
};

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
    /// Back reference to the user page pool.
    upool: Rc<RefCell<UpoolInner>>,
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
    fn new(addr: FrameAddress, upool: Rc<RefCell<UpoolInner>>) -> Self {
        Self { addr, upool }
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

impl Deref for UserFrame {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(self.addr.into_raw_value() as *const u8, mem::PAGE_SIZE)
        }
    }
}

impl DerefMut for UserFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            core::slice::from_raw_parts_mut(self.addr.into_raw_value() as *mut u8, mem::PAGE_SIZE)
        }
    }
}

impl Drop for UserFrame {
    fn drop(&mut self) {
        if let Err(err) = self.upool.borrow_mut().free(self.addr) {
            error!("failed to free user frame: {:?}", err);
        }
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
        let upage: UserFrame = UserFrame::new(addr, self.inner.clone());
        // TODO: clear user page.
        Ok(upage)
    }

    pub fn alloc_many(&mut self, nframes: usize) -> Result<Vec<UserFrame>, Error> {
        trace!("alloc_many(): size={}", nframes);

        // Attempt to allocate pages.
        let mut pages: Vec<FrameAddress> = self.inner.borrow_mut().alloc_many(nframes)?;

        // Create a vector of user pages.
        let mut upages: Vec<UserFrame> = Vec::new();
        while let Some(page) = pages.pop() {
            let upage: UserFrame = UserFrame::new(page, self.inner.clone());
            // TODO: clear user page.
            upages.push(upage);
        }

        Ok(upages)
    }
}
