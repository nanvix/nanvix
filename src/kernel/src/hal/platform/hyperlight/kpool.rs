// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Hyperlight-side kernel page pool.
//!
//! Two allocation shapes, one sparse bitmap that tracks both.
//!
//! # Single-page allocations (`alloc`)
//!
//! Backing: **scratch**. The bump cursor at `ALLOCATOR_GVA` advances
//! one page, and we book the resulting GPA in the pre-provisioned
//! scratch chunk of the sparse bitmap.
//!
//! Why scratch: these pages are overwhelmingly used as page tables.
//! The hardware MMU walks using the physical addresses stored in PDEs
//! — if a PT's PA lives in the snapshot region (EPT-RO), the CPU
//! reads stale snapshot bytes, not the live CoW'd copy. Putting PTs
//! in scratch makes the PA itself point at live content, so both the
//! hardware and Nanvix's software walkers agree.
//!
//! # Multi-page allocations (`alloc_range`)
//!
//! Backing: **dense RAM** from the snapshot's kpool region. This is
//! the same as the microvm path. The only caller today is
//! `KernelStack::new`, and a kstack is never walked as a page-table
//! structure — it's accessed by VA through the CoW identity map.
//!
//! Why *not* scratch for kstacks: the snapshot path filters out scratch
//! VAs (see the architecture doc §7b), and restore zeroes scratch
//! (§8). The pre-forged `iretl` frame that `forge_user_context` writes
//! onto a freshly-allocated kstack must survive snapshot/restore; that
//! survival is what the CoW+compaction machinery already provides for
//! kernel-low VAs, and only for kernel-low VAs. Putting the kstack in
//! scratch would drop its mapping at snapshot and wipe its content at
//! restore, so the first ring-3→0 transition would pop zeros and fault
//! at eip=0.
//!
//! # Bitmap layout
//!
//! Two chunks, both pre-provisioned at `Inner::new`:
//! - Dense RAM chunk at offset `region.start() / PAGE_SIZE`, one bit
//!   per frame of `region`.
//! - Scratch chunk at offset `scratch_base_gpa / PAGE_SIZE`, one bit
//!   per frame of the scratch region (see
//!   [`crate::mm::Vmem::scratch_range_in_frames`]).
//!
//! Both ranges are indexed by global frame number, so `free` is
//! symmetric: one `bitmap.clear(addr / PAGE_SIZE)`, no underflow, no
//! cosmetic teardown errors.

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

#[derive(Debug)]
pub struct Inner {
    /// RAM kpool region provided by the platform at boot. Backs
    /// `alloc_range` (kstacks).
    #[allow(dead_code)]
    region: TruncatedMemoryRegion<PhysicalAddress>,
    /// Two-chunk sparse bitmap: one chunk for the dense RAM region,
    /// one for the scratch region. Shape is fixed at construction.
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
        let dense_offset: usize = region.start().into_raw_value() / mem::PAGE_SIZE;

        // Scratch chunk covering the whole scratch region. Pre-provisioned
        // at init so `alloc()` can just `bitmap.set(frame_index)` without
        // ever needing to grow the bitmap — per the sparse-bitmap contract,
        // the chunk layout is fixed after construction.
        let (scratch_offset, scratch_frames) = crate::mm::Vmem::scratch_range_in_frames();
        let scratch_bitmap: Bitmap = Bitmap::new(scratch_frames)?;

        let bitmap: SparseBitmap = SparseBitmap::new(::alloc::vec![
            (dense_offset, dense),
            (scratch_offset, scratch_bitmap),
        ])?;
        Ok(Self { region, bitmap })
    }

    /// Single-page allocation: scratch-backed. The PA is in the
    /// scratch region so page tables built from these frames are
    /// walked (by hardware and software) from live content. The
    /// scratch chunk was pre-provisioned by `Self::new`, so the
    /// `bitmap.set` never needs to grow state.
    pub fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let frame: FrameAddress = crate::mm::Vmem::alloc_scratch_frame()?;
        let index: usize = frame.into_raw_value() / mem::PAGE_SIZE;
        self.bitmap.set(index)?;
        Ok(frame)
    }

    /// Multi-page contiguous allocation: dense RAM. The CoW
    /// mechanism redirects writes through scratch transparently and
    /// the snapshot path preserves content across restore. See the
    /// module comment for the full rationale.
    pub fn alloc_range(&mut self, count: usize) -> Result<Vec<FrameAddress>, Error> {
        let index: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };
        let base: usize = index * mem::PAGE_SIZE;
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
        let index: usize = addr.into_raw_value() / mem::PAGE_SIZE;
        match self.bitmap.clear(index) {
            Ok(()) => Ok(()),
            Err(error) => {
                error!("{error:?} (addr={addr:?})");
                Err(error)
            },
        }
    }

}
