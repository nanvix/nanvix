// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Frame allocator — module-level singleton.
//!
//! The frame allocator is backed by a [`SparseBitmap`] and exposed as free functions over a
//! singleton so every in-kernel caller (upool, kpool, anything else that needs a raw frame) goes
//! through the same state. No struct-valued handle is passed around.
//!
//! Access to the frame allocator is synchronized externally and performed by a single thread, so
//! the backing bitmap uses non-atomic operations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::{
    FrameAddress,
    PageAligned,
    PhysicalAddress,
    TruncatedMemoryRegion,
};
use ::arch::mem::{
    self,
    paging::FrameNumber,
};
use ::config::constants;
use ::core::{
    hint::unlikely,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sparse_bitmap::SparseBitmap;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::Address,
};
use ::vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("frame.spec.rs");

#[cfg(verus_keep_ghost)]
include!("frame.proof.rs");

//==================================================================================================
// Inner
//==================================================================================================

/// Private state of the frame allocator singleton.
#[verus_verify]
struct Inner {
    /// A sparse bitmap that keeps track of free/used frames.
    bitmap: SparseBitmap,
}

#[verus_verify]
impl Inner {
    ///
    /// # Description
    ///
    /// Allocates a frame.
    ///
    /// # Returns
    ///
    /// Upon success, the address of the allocated frame is returned. Upon failure, an error is
    /// returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(frame) => {
                    &&& frame.inv()
                    &&& old(self)@.free_frames.contains(frame@)
                    &&& self@ == old(self)@.spec_alloc(frame@)
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& old(self)@.free_frames.is_empty()
                }
            },
    )]
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        proof! { admit(); }
        let frame_number: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                return Err(error);
            },
        };
        let frame_number: FrameNumber = match FrameNumber::from_raw_value(frame_number) {
            Some(frame_number) => frame_number,
            None => {
                let reason: &str = "frame number is out of bounds";
                #[cfg(not(verus_keep_ghost))]
                error!("{reason:?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Attempt to convert the frame number to a frame address.
        match FrameAddress::from_frame_number(frame_number) {
            Ok(frame_address) => Ok(frame_address),
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?}");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Frees a frame that was previously allocated.
    ///
    /// # Parameters
    ///
    /// - `frame`: Address of the frame to free.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            frame.inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.allocated_frames.contains(frame@)
                    &&& self@ == old(self)@.spec_free(frame@)
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& !old(self)@.allocated_frames.contains(frame@)
                }
            },
    )]
    fn free(&mut self, frame: FrameAddress) -> Result<(), Error> {
        proof! { admit(); }
        let frame_number: usize = frame.into_frame_number().into_raw_value();
        match self.bitmap.clear(frame_number) {
            Ok(()) => Ok(()),
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (frame={frame:?})");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Books a frame so that it will not be handed out by [`alloc`].
    ///
    /// # Parameters
    ///
    /// - `phys_addr`: Physical address of the frame to book.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            phys_addr.inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(self)@.free_frames.contains(phys_addr@)
                    &&& self@ == old(self)@.spec_book(phys_addr@)
                },
                Err(_) => {
                    &&& self@ == old(self)@
                    &&& !old(self)@.free_frames.contains(phys_addr@)
                }
            },
    )]
    fn book(&mut self, phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
        proof! { admit(); }
        let frame_number: usize = phys_addr.into_frame_number().into_raw_value();
        match self.bitmap.set(frame_number) {
            Ok(()) => Ok(()),
            Err(error) => {
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (phys_addr={phys_addr:?})");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Allocates all frames in the given region.
    ///
    /// # Parameters
    ///
    /// - `region`: Physical memory region whose frames should be booked.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_spec(result =>
        requires
            old(self).inv(),
            region.inv(),
        ensures
            self.inv(),
            ({
                let start_frame_number = region@.start / spec_page_size();
                let end_frame_number = (region@.start + region@.size) / spec_page_size();
                let frame_numbers = vstd::set_lib::set_int_range(start_frame_number, end_frame_number);
                let frames = frame_numbers.map(|i: int| i * spec_page_size());
                match result {
                    Ok(()) => {
                        &&& frames.subset_of(old(self)@.free_frames)
                        &&& self@ == old(self)@.spec_alloc_range(frames)
                    },
                    Err(_) => {
                        &&& self@ == old(self)@
                        &&& !frames.subset_of(old(self)@.free_frames)
                    },
                }
            }),
    )]
    fn alloc_range(
        &mut self,
        region: &TruncatedMemoryRegion<PhysicalAddress>,
    ) -> Result<(), Error> {
        proof! { admit(); }
        let start_frame_number: usize = region.start().into_frame_number().into_raw_value();
        // VERUS REWRITE: replaced `start + size/FRAME_SIZE - 1` and `..=` (inclusive range)
        // with exclusive upper bound. RangeInclusive<usize> lacks ForLoopGhostIteratorNew.
        let num_frames: usize = region.size() / mem::FRAME_SIZE;
        let end_frame_number: usize = start_frame_number + num_frames;

        // When nightly-performance-optimizations is off, verify that every frame index in the
        // range is covered by the sparse bitmap. SparseBitmap::test() returns Ok(false) for
        // uncovered indices, which would incorrectly appear as "free" and pass the check below,
        // only to fail on set(). With the feature enabled this check is elided because
        // PhysicalAddress construction already guarantees valid physical addresses.
        #[cfg(not(feature = "nightly-performance-optimizations"))]
        #[verus_spec(invariant(true))]
        for index in start_frame_number..end_frame_number {
            proof! { admit(); }
            if self.bitmap.find_chunk(index).is_none() {
                let uncovered_addr: usize = index * mem::FRAME_SIZE;
                let reason: &str = "frame index not covered by any bitmap chunk";
                #[cfg(not(verus_keep_ghost))]
                error!("{} (frame={:#010x}, region={:?})", reason, uncovered_addr, region);
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }
        }

        // Check if all frames in the range are free.
        #[verus_spec(invariant(true))]
        for index in start_frame_number..end_frame_number {
            proof! { admit(); }
            match self.bitmap.test(index) {
                Ok(false) => {
                    // Frame is free — nothing to do.
                },
                Ok(true) => {
                    let conflicting_addr: usize = index * mem::FRAME_SIZE;
                    let region_start: usize = region.start().into_raw_value();
                    let region_end: usize = region_start.saturating_add(region.size());
                    let reason: &str = "frame is already allocated";
                    #[cfg(not(verus_keep_ghost))]
                    error!(
                        "{} (frame={:#010x}, region_start={:#010x}, region_end={:#010x})",
                        reason, conflicting_addr, region_start, region_end
                    );
                    return Err(Error::new(ErrorCode::OutOfMemory, reason));
                },
                Err(err) => return Err(err),
            }
        }

        // Book all frames in the range.
        #[verus_spec(invariant(true))]
        for index in start_frame_number..end_frame_number {
            proof! { admit(); }
            if let Err(error) = self.bitmap.set(index) {
                #[cfg(not(verus_keep_ghost))]
                error!("{error:?} (region={region:?})");
                return Err(error);
            }
        }

        Ok(())
    }
}

//==================================================================================================
// Constants
//==================================================================================================

// Use relaxed ordering for all atomic operations to mitigate synchronization overhead. It is safe
// to use this ordering semantics because Nanvix is a single-core system, and the kernel runs with
// interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Singleton
//==================================================================================================

/// Module-level singleton storage.
static mut INSTANCE: MaybeUninit<Inner> = MaybeUninit::uninit();

/// Whether the frame allocator has been initialized.
static INSTANCE_INIT: AtomicBool = AtomicBool::new(false);

/// Returns a mutable reference to the initialized singleton.
fn instance() -> &'static mut Inner {
    if unlikely(!INSTANCE_INIT.load(ORDER)) {
        panic!("frame allocator used before init()");
    }

    // SAFETY: `INSTANCE_INIT` is `true`, so `INSTANCE` has been fully written by `init()`.
    // The kernel is single-threaded with interrupts disabled, so no concurrent access is possible.
    unsafe { INSTANCE.assume_init_mut() }
}

//==================================================================================================
// Public Free Functions
//==================================================================================================

/// Initialize the frame allocator singleton.
///
/// # Safety
///
/// Must be called exactly once during boot, before any other function
/// in this module.
pub(super) unsafe fn init(bitmap: SparseBitmap) -> Result<(), Error> {
    if unlikely(INSTANCE_INIT.load(ORDER)) {
        return Err(Error::new(ErrorCode::InvalidArgument, "frame allocator already initialized"));
    }

    #[cfg(not(verus_keep_ghost))]
    info!(
        "frame allocator: {} frames, {} MB, {} chunk(s)",
        bitmap.capacity(),
        (bitmap.capacity() * mem::FRAME_SIZE) / constants::MEGABYTE,
        bitmap.chunk_count(),
    );

    // SAFETY: single-threaded boot; no other reference to `INSTANCE` exists.
    unsafe { INSTANCE.write(Inner { bitmap }) };
    INSTANCE_INIT.store(true, ORDER);
    Ok(())
}

/// Allocate a frame.
/// Singleton pattern: state transition tracked by Inner::alloc.
#[verus_spec(result =>
    ensures
        match result {
            Ok(frame) => frame.inv(),
            // Singleton pattern: cannot express state-preservation without ghost accessor.
            Err(_) => true,
        },
)]
pub(super) fn alloc() -> Result<FrameAddress, Error> {
    instance().alloc()
}

// NOTE: free uses verus! syntax because Drop::drop requires `no_unwind`,
// and the attribute-based syntax does not support `no_unwind`.
verus! {
/// Free a frame previously returned by [`alloc`].
#[verifier::external_body]
pub(super) fn free(frame: FrameAddress) -> (result: Result<(), Error>)
    requires
        frame.inv(),
    ensures
        // Singleton pattern: state transition tracked by Inner::free.
        result.is_ok() || result.is_err(),
    opens_invariants none
    no_unwind
{
    instance().free(frame)
}
}

/// Reserve a frame so [`alloc`] will skip it.
/// Singleton pattern: state transition tracked by Inner::book.
#[verus_spec(result =>
    requires
        phys_addr.inv(),
    ensures
        // Singleton pattern: cannot express state transition without ghost accessor.
        result.is_ok() || result.is_err(),
)]
pub(super) fn book(phys_addr: PageAligned<PhysicalAddress>) -> Result<(), Error> {
    instance().book(phys_addr)
}

/// Book every frame in the given physical memory region.
/// Singleton pattern: state transition tracked by Inner::alloc_range.
#[verus_spec(result =>
    requires
        region.inv(),
    ensures
        // Singleton pattern: cannot express state transition without ghost accessor.
        result.is_ok() || result.is_err(),
)]
pub(super) fn alloc_range(region: &TruncatedMemoryRegion<PhysicalAddress>) -> Result<(), Error> {
    instance().alloc_range(region)
}
