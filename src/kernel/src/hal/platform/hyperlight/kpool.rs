// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Hyperlight-side kernel page pool implementation.
//!
//! Mirrors the microvm implementation: hands out frames from a fixed
//! pool region tracked by a single-chunk [`SparseBitmap`]. The
//! Hyperlight-specific scratch-bump strategy lands together with the
//! rest of the Nanvix-on-Hyperlight feature; until then this module
//! exists so the platform dispatch in `mm/phys/{kpool,frame}.rs`
//! resolves cleanly when the kernel is built with `--features hyperlight`.
//!
//! FIXME: temporary scaffolding. Replaced wholesale by the scratch-backed
//! implementation when Nanvix-on-Hyperlight (CoW snapshot/restore) lands
//! upstream; at that point the kpool draws from the scratch bump cursor
//! and books frames via [`SparseBitmap::set_or_add_chunk`] rather than
//! sharing the microvm dense-region path.

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
        let index: usize = (addr.into_raw_value() - self.region.start().into_raw_value())
            / mem::PAGE_SIZE;
        match self.bitmap.clear(index) {
            Ok(()) => Ok(()),
            Err(error) => {
                error!("{error:?} (addr={addr:?})");
                Err(error)
            },
        }
    }
}
