// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::Bitmap,
    hal::mem::{
        Address,
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
};
use ::alloc::{
    rc::Rc,
    vec::Vec,
};
use ::arch::mem;
use ::core::{
    cell::RefCell,
    ops::{
        Deref,
        DerefMut,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};

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
        trace!("region={region:?}");
        debug_assert_eq!(
            region.size() % mem::PAGE_SIZE,
            0,
            "kernel pool size must be a multiple of page size"
        );
        let bitmap: Bitmap = Bitmap::new(region.size() / (mem::PAGE_SIZE))?;
        Ok(Self { region, bitmap })
    }

    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let index: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?}");
                return Err(error);
            },
        };
        let addr: usize = self.region.start().into_raw_value() + index * mem::PAGE_SIZE;
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?))
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of frame addresses from the kernel pool.
    ///
    /// # Parameters
    ///
    /// - `addrs`: Mutable reference to a pre-allocated vector. The number of frames allocated
    ///   equals `addrs.capacity()`.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `addrs` is filled to capacity with contiguous
    /// entries. Upon failure, an error is returned instead.
    ///
    fn alloc_range(&mut self, addrs: &mut Vec<FrameAddress>) -> Result<(), Error> {
        if !addrs.is_empty() {
            let reason: &str = "addrs vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let count: usize = addrs.capacity();
        let index: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };

        let base_addr: usize = self.region.start().into_raw_value() + index * mem::PAGE_SIZE;
        for i in 0..count {
            let addr: usize = base_addr + i * mem::PAGE_SIZE;
            let frame: FrameAddress = FrameAddress::new(PageAligned::from_address(
                PhysicalAddress::from_raw_value(addr)?,
            )?);
            addrs.push(frame);
        }

        Ok(())
    }

    /// Frees a page in the kernel pool.
    fn free(&mut self, addr: FrameAddress) -> Result<(), Error> {
        let index: usize =
            (addr.into_raw_value() - self.region.start().into_raw_value()) / mem::PAGE_SIZE;
        match self.bitmap.clear(index) {
            Ok(()) => Ok(()),
            Err(error) => {
                error!("{error:?} (addr={addr:?})");
                Err(error)
            },
        }
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
    /// Clears the target kernel frame.
    ///
    pub fn clear(&mut self) {
        self.deref_mut().fill(0);
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
    /// # Return Values
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc(&mut self) -> Result<KernelFrame, Error> {
        let frame: FrameAddress = self.inner.borrow_mut().alloc()?;
        Ok(KernelFrame::new(self.inner.clone(), frame))
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel frames from the kernel frame pool.
    ///
    /// # Parameters
    ///
    /// - `frames`: Mutable reference to a pre-allocated vector. The number of frames allocated
    ///   equals `frames.capacity()`.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `frames` is filled to capacity with contiguous
    /// entries. Upon failure, an error is returned instead.
    ///
    pub fn alloc_many(&mut self, frames: &mut Vec<KernelFrame>) -> Result<(), Error> {
        // Check if caller-provided vector is not empty.
        if !frames.is_empty() {
            let reason: &str = "frames vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let count: usize = frames.capacity();
        let mut addrs: Vec<FrameAddress> = Vec::with_capacity(count);
        self.inner.borrow_mut().alloc_range(&mut addrs)?;
        for addr in addrs {
            frames.push(KernelFrame::new(self.inner.clone(), addr));
        }
        Ok(())
    }
}
