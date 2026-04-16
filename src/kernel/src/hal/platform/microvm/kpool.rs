// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Microvm-side kernel page pool implementation.
//!
//! Hands out frames from a fixed pool region. Allocation state is a
//! single-chunk [`SparseBitmap`] indexed locally to the pool
//! (`[0, num_frames)`); using the sparse representation here keeps the
//! shape uniform with other platforms (e.g. Hyperlight) that grow chunks
//! on demand from a foreign address space.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::{
        Bitmap,
        SparseBitmap,
    },
    hal::mem::{
        Address,
        FrameAddress,
        PageAligned,
        PhysicalAddress,
        TruncatedMemoryRegion,
    },
};
use ::alloc::vec::Vec;
use ::arch::mem;
use ::sys::error::Error;

//==================================================================================================
// Inner
//==================================================================================================

/// Microvm-side kpool inner state. Owned by [`crate::mm::phys::Kpool`]
/// inside an `Rc<RefCell<_>>`.
#[derive(Debug)]
pub struct Inner {
    region: TruncatedMemoryRegion<PhysicalAddress>,
    bitmap: SparseBitmap,
}

impl Inner {
    pub fn new(region: TruncatedMemoryRegion<PhysicalAddress>) -> Result<Self, Error> {
        trace!("region={region:?}");
        debug_assert_eq!(
            region.size() % mem::PAGE_SIZE,
            0,
            "kernel pool size must be a multiple of page size"
        );
        let dense: Bitmap = Bitmap::new(region.size() / mem::PAGE_SIZE)?;
        let bitmap: SparseBitmap = SparseBitmap::new(::alloc::vec![(0, dense)])?;
        Ok(Self { region, bitmap })
    }

    pub fn alloc(&mut self) -> Result<FrameAddress, Error> {
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

    pub fn alloc_range(&mut self, count: usize) -> Result<Vec<FrameAddress>, Error> {
        let index: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };
        let base: usize = self.region.start().into_raw_value() + index * mem::PAGE_SIZE;
        let mut pages: Vec<FrameAddress> = Vec::with_capacity(count);
        for i in 0..count {
            let addr: usize = base + i * mem::PAGE_SIZE;
            pages.push(FrameAddress::new(PageAligned::from_address(
                PhysicalAddress::from_raw_value(addr)?,
            )?));
        }
        Ok(pages)
    }

    pub fn free(&mut self, addr: FrameAddress) -> Result<(), Error> {
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
