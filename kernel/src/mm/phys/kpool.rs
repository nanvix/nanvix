// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    hal::mem::{
        Address,
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
    klib::bitmap::Bitmap,
};
use ::alloc::{
    rc::Rc,
    vec::Vec,
};
use ::core::{
    cell::RefCell,
    ops::{
        Deref,
        DerefMut,
    },
};
use ::sys::error::Error;

//==================================================================================================
// Kernel Page Pool Inner
//==================================================================================================

#[derive(Debug)]
struct KpoolInner {
    /// Size of the kernel pool.
    region: TruncatedMemoryRegion<PhysicalAddress>,
    /// Bitmap of free pages.
    bitmap: Bitmap,
}

impl KpoolInner {
    fn new(region: TruncatedMemoryRegion<PhysicalAddress>) -> Result<Self, Error> {
        let bitmap: Bitmap = Bitmap::new(region.size() / (mem::PAGE_SIZE * u8::BITS as usize))?;
        Ok(Self { region, bitmap })
    }

    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let index: usize = self.bitmap.alloc()?;
        let addr: usize = self.region.start().into_raw_value() + index * mem::PAGE_SIZE;
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?))
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of pages in the kernel page pool.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of frames to allocate.
    ///
    /// # Returns
    ///
    /// Upon success, a vector of page-aligned addresses is returned. Upon failure, an error code is
    /// returned instead.
    ///
    fn alloc_range(&mut self, count: usize) -> Result<Vec<FrameAddress>, Error> {
        // Attempt to allocate a range of pages.
        let index: usize = self.bitmap.alloc_range(count)?;

        // Create a vector of page-aligned addresses.
        let base_addr: usize = self.region.start().into_raw_value() + index * mem::PAGE_SIZE;
        let mut pages: Vec<FrameAddress> = Vec::new();
        for i in 0..count {
            let addr: usize = base_addr + i * mem::PAGE_SIZE;
            let page: FrameAddress = FrameAddress::new(PageAligned::from_address(
                PhysicalAddress::from_raw_value(addr)?,
            )?);
            pages.push(page);
        }

        Ok(pages)
    }

    /// Frees a page in the kernel pool.
    fn free(&mut self, addr: FrameAddress) -> Result<(), Error> {
        let index: usize =
            (addr.into_raw_value() - self.region.start().into_raw_value()) / mem::PAGE_SIZE;
        self.bitmap.clear(index)
    }
}

//==================================================================================================
// Kernel Page
//==================================================================================================

#[derive(Debug)]
pub struct KernelFrame {
    kpool: Rc<RefCell<KpoolInner>>,
    base: FrameAddress,
}

impl KernelFrame {
    fn new(kpool: Rc<RefCell<KpoolInner>>, base: FrameAddress) -> Self {
        Self { kpool, base }
    }

    pub fn base(&self) -> FrameAddress {
        self.base
    }

    ///
    /// # Description
    ///
    /// Clears the target kernel page.
    ///
    fn clear(&mut self) {
        for byte in self.iter_mut() {
            *byte = 0;
        }
    }
}

impl Deref for KernelFrame {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(self.base.into_raw_value() as *const u8, mem::PAGE_SIZE)
        }
    }
}

impl DerefMut for KernelFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            core::slice::from_raw_parts_mut(self.base.into_raw_value() as *mut u8, mem::PAGE_SIZE)
        }
    }
}

impl Drop for KernelFrame {
    fn drop(&mut self) {
        trace!("freeing kernel page: {:?}", self.base);
        if let Err(e) = self.kpool.borrow_mut().free(self.base) {
            error!("failed to free kernel page pool: {:?}", e)
        }
    }
}

//==================================================================================================
// Kernel Pool
//==================================================================================================

#[derive(Debug)]
pub struct Kpool {
    inner: Rc<RefCell<KpoolInner>>,
}

impl Kpool {
    /// Initializes the kernel pool.
    pub fn new(region: TruncatedMemoryRegion<PhysicalAddress>) -> Result<Self, Error> {
        Ok(Self {
            inner: Rc::new(RefCell::new(KpoolInner::new(region)?)),
        })
    }

    ///
    /// # Description
    ///
    /// Allocates a kernel frame from the kernel frame pool.
    ///
    /// # Parameters
    ///
    /// - `clear`: Clear page?
    ///
    /// # Return Values
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc(&mut self, clear: bool) -> Result<KernelFrame, Error> {
        let frame: FrameAddress = self.inner.borrow_mut().alloc()?;
        let mut kframe: KernelFrame = KernelFrame::new(self.inner.clone(), frame);
        if clear {
            // TODO: move clear logic to page-level.
            kframe.clear();
        }
        Ok(kframe)
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of frames from the kernel frame pool.
    ///
    /// # Parameters
    ///
    /// - `clear`: Clear pages?
    /// - `count`: Number of pages to allocate.
    ///
    /// # Return Values
    ///
    /// Upon success, a vector of kernel frames is returned. Upon failure, an error is returned
    /// instead.
    ///
    pub fn alloc_many(&mut self, clear: bool, count: usize) -> Result<Vec<KernelFrame>, Error> {
        // Attempt to allocate pages.
        let mut kframes: Vec<FrameAddress> = self.inner.borrow_mut().alloc_range(count)?;

        // Create a vector of kernel pages.
        let mut kpages: Vec<KernelFrame> = Vec::new();
        while let Some(kframe) = kframes.pop() {
            let mut kframe: KernelFrame = KernelFrame::new(self.inner.clone(), kframe);
            if clear {
                // TODO: move clear logic to page-level.
                kframe.clear();
            }
            kpages.push(kframe);
        }

        Ok(kpages)
    }
}
