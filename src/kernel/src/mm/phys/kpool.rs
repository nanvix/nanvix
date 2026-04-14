// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Platform-agnostic facade over the kernel page pool. The inner state
//! type lives in [`crate::hal::platform::kpool::Inner`] (per platform);
//! this module wraps it in the shared `Rc<RefCell<_>>` shape that
//! callers and `KernelFrame::Drop` rely on for safe re-entrant access.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::{
    mem::{
        FrameAddress,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
    platform::kpool::Inner,
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
use ::sys::error::Error;

//==================================================================================================
// Kernel Frame
//==================================================================================================

#[derive(Debug)]
pub struct KernelFrame {
    kpool: Rc<RefCell<Inner>>,
    base: FrameAddress,
}

impl KernelFrame {
    fn new(kpool: Rc<RefCell<Inner>>, base: FrameAddress) -> Self {
        Self { kpool, base }
    }

    pub fn base(&self) -> FrameAddress {
        self.base
    }

    fn clear(&mut self) {
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
            error!("failed to free kernel page pool: {e:?}");
        }
    }
}

//==================================================================================================
// Kernel Pool
//==================================================================================================

#[derive(Debug)]
pub struct Kpool {
    inner: Rc<RefCell<Inner>>,
}

impl Kpool {
    pub fn new(region: TruncatedMemoryRegion<PhysicalAddress>) -> Result<Self, Error> {
        Ok(Self {
            inner: Rc::new(RefCell::new(Inner::new(region)?)),
        })
    }

    pub fn alloc(&mut self, clear: bool) -> Result<KernelFrame, Error> {
        let frame: FrameAddress = self.inner.borrow_mut().alloc()?;
        let mut kframe: KernelFrame = KernelFrame::new(self.inner.clone(), frame);
        if clear {
            kframe.clear();
        }
        Ok(kframe)
    }

    pub fn alloc_many(&mut self, clear: bool, count: usize) -> Result<Vec<KernelFrame>, Error> {
        let mut kframes: Vec<FrameAddress> = self.inner.borrow_mut().alloc_range(count)?;
        let mut kpages: Vec<KernelFrame> = Vec::new();
        while let Some(kframe) = kframes.pop() {
            let mut kframe: KernelFrame = KernelFrame::new(self.inner.clone(), kframe);
            if clear {
                kframe.clear();
            }
            kpages.push(kframe);
        }
        Ok(kpages)
    }
}
