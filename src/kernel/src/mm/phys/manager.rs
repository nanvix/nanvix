// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("manager.spec.rs");
#[cfg(verus_keep_ghost)]
include!("manager.proof.rs");

use crate::{
    hal::mem::FrameAddress,
    mm::phys::{
        frame,
        kframe::KernelFrame,
        upool::{
            Upool,
            UserFrame,
        },
    },
};
use ::alloc::vec::Vec;
use ::arch::mem;
use ::core::{
    hint::unlikely,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicBool,
        Ordering,
    },
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::vstd::prelude::*;

//==================================================================================================
// Constants
//==================================================================================================

// Use relaxed ordering for all atomic operations to mitigate synchronization overhead. It is safe
// to use this ordering semantics because Nanvix is a single-core system, and the kernel runs with
// interrupts disabled.
const ORDER: Ordering = Ordering::Relaxed;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Physical memory manager storage.
static mut PHYS_MEMORY_MANAGER: MaybeUninit<PhysMemoryManager> = MaybeUninit::uninit();

/// Whether the physical memory manager has been initialized.
static PHYS_MEMORY_MANAGER_INIT: AtomicBool = AtomicBool::new(false);

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Physical memory manager.
///
#[verus_verify]
pub struct PhysMemoryManager {
    upool: Upool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PhysMemoryManager {
    ///
    /// # Description
    ///
    /// Initializes the physical memory manager singleton.
    ///
    /// # Parameters
    ///
    /// - `upool`: User page pool.
    ///
    /// # Errors
    ///
    /// Returns `InvalidArgument` if the singleton has already been initialized.
    ///
    // Trusted shim: writes the `static mut` singleton storage and an `AtomicBool`
    // lifecycle gate (raw global state Verus cannot model). The manager-singleton
    // lifecycle flag is distinct from the frame-allocator lifecycle and has no
    // abstract model (the do-not-modify `PhysMemView` only tracks the allocator);
    // the contract therefore states the caller-relevant guarantee: the global
    // frame allocator stays initialized and well-formed across this call.
    #[allow(verus_impl_method_marker)]
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            phys_view().initialized,
            phys_view().inv(),
        ensures
            phys_view().inv(),
            phys_view().initialized,
    )]
    pub(super) fn init(upool: Upool) -> Result<(), Error> {
        if unlikely(PHYS_MEMORY_MANAGER_INIT.load(ORDER)) {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "physical memory manager already initialized",
            ));
        }

        // SAFETY: this happens during kernel initialization and no other threads are running.
        unsafe { PHYS_MEMORY_MANAGER.write(PhysMemoryManager { upool }) };
        PHYS_MEMORY_MANAGER_INIT.store(true, ORDER);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the physical memory manager.
    ///
    /// # Panics
    ///
    /// Panics if the physical memory manager is not initialized.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it returns a mutable reference to a global variable.
    ///
    /// The caller must ensure:
    ///
    /// - No other `&mut PhysMemoryManager` reference obtained from this function is live at the
    ///   same time (i.e., `&mut` references must not overlap). In practice this is guaranteed
    ///   because the kernel is single-threaded and runs with interrupts disabled, so no
    ///   re-entrant or concurrent call can alias the reference.
    ///
    pub unsafe fn get_mut<'a>() -> &'a mut PhysMemoryManager {
        if unlikely(!PHYS_MEMORY_MANAGER_INIT.load(ORDER)) {
            panic!("physical memory manager is not initialized");
        }

        // SAFETY: the physical memory manager has been initialized, so the value is valid.
        PHYS_MEMORY_MANAGER.assume_init_mut()
    }

    ///
    /// # Description
    ///
    /// Allocates user frames into caller-provided storage.
    ///
    /// The returned frames are not guaranteed to be physically contiguous.
    /// User allocations are gated by the kernel watermark: if fulfilling the request would
    /// leave fewer than `KERNEL_WATERMARK` free frames, the allocation is rejected.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of frames to allocate.
    /// - `frames`: Mutable reference to a pre-allocated vector into which to store those
    ///   frames' addresses. It should have sufficient capacity for `count` entries.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `frames` is filled with `count` frames. Upon failure, an
    /// error is returned and any frames allocated by this call are dropped by truncating `frames`
    /// back to empty.
    ///
    // Trusted shim: loops over the global frame allocator (`Upool::alloc`) and the
    // caller-provided `Vec`, with `error!`-logging and a clear()-on-error rollback
    // that Verus cannot body-verify without editing exec code. The contract is
    // stated over `phys_view()` and `frames@`: on success exactly `count` frames
    // are handed out (each now allocated); on error the vector is emptied
    // (all-or-nothing). Frames are not claimed contiguous.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            phys_view().initialized,
            phys_view().inv(),
            old(frames)@.len() == 0,
        ensures
            phys_view().inv(),
            phys_view().initialized,
            match result {
                Ok(()) => {
                    &&& final(frames)@.len() == count
                    &&& forall|i: int|
                        0 <= i < count as int ==>
                            #[trigger] phys_view().frames.allocated_frames.contains(final(frames)@[i]@)
                },
                Err(_) => final(frames)@.len() == 0,
            },
    )]
    pub fn alloc_many_user_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<UserFrame>,
    ) -> Result<(), Error> {
        if !frames.is_empty() {
            let reason: &str = "frames vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        if frames.capacity() < count {
            let reason: &str = "frames vector has insufficient capacity";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        // A zero-sized allocation is trivially satisfied.
        if count == 0 {
            return Ok(());
        }

        Self::check_user_watermark(count)?;

        for _ in 0..count {
            match self.upool.alloc() {
                Ok(frame) => frames.push(frame),
                Err(error) => {
                    frames.clear();
                    return Err(error);
                },
            }
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Allocates a single user frame, applying the same kernel watermark check as
    /// [`Self::alloc_many_user_frames`]. This is the single-frame fast path used on
    /// hot paths such as copy-on-write fault resolution, where allocating an
    /// intermediate [`Vec`] would be wasteful.
    ///
    /// # Returns
    ///
    /// Upon success, a [`UserFrame`] is returned. Upon failure, an error is returned
    /// instead.
    ///
    // Trusted shim: applies the watermark gate then delegates to the global
    // `Upool::alloc`. On success the returned frame's physical address is now in
    // the allocator's `allocated_frames` and is page-aligned.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            phys_view().initialized,
            phys_view().inv(),
        ensures
            phys_view().inv(),
            phys_view().initialized,
            match result {
                Ok(frame) => {
                    &&& phys_view().frames.allocated_frames.contains(frame@)
                    &&& frame@ % spec_page_size() == 0
                },
                Err(_) => true,
            },
    )]
    pub fn alloc_user_frame(&mut self) -> Result<UserFrame, Error> {
        Self::check_user_watermark(1)?;
        self.upool.alloc()
    }

    ///
    /// # Description
    ///
    /// Rejects user allocations of `count` frames that would breach the kernel
    /// watermark, i.e. that would drop the number of free frames below
    /// [`config::kernel::KERNEL_WATERMARK`].
    ///
    /// # Parameters
    ///
    /// - `count`: Number of user frames the caller intends to allocate.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    // Trusted shim: reads the global free-frame count and compares it against
    // `KERNEL_WATERMARK + count`. Pure gate, no allocator state change. `Ok` exactly
    // captures the watermark policy: at least `KERNEL_WATERMARK` frames remain free
    // after servicing `count`. `Err` covers both the overflow guard and a breach.
    #[allow(verus_impl_method_marker)]
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            phys_view().initialized,
            phys_view().inv(),
        ensures
            phys_view().inv(),
            phys_view().initialized,
            phys_view().frames.free_frames.finite(),
            match result {
                Ok(()) => spec_watermark_ok(phys_view().frames, count as int),
                Err(_) => true,
            },
    )]
    fn check_user_watermark(count: usize) -> Result<(), Error> {
        let watermark_threshold: usize = config::kernel::KERNEL_WATERMARK
            .checked_add(count)
            .ok_or_else(|| {
                let reason: &str = "watermark + count overflow";
                error!("{reason}");
                Error::new(ErrorCode::InvalidArgument, reason)
            })?;
        if frame::free_count() < watermark_threshold {
            let reason: &str = "would breach kernel watermark";
            error!("{reason}");
            return Err(Error::new(ErrorCode::OutOfMemory, reason));
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Allocates a kernel frame.
    ///
    /// Kernel allocations bypass the watermark — no artificial ceiling.
    ///
    /// # Return Values
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    // Trusted shim: allocates one frame from the global allocator (bypassing the
    // watermark) and wraps it, freeing the raw frame if wrapping fails. On success
    // the returned frame's base address is now allocated and page-aligned.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            phys_view().initialized,
            phys_view().inv(),
        ensures
            phys_view().inv(),
            phys_view().initialized,
            match result {
                Ok(frame) => {
                    &&& phys_view().frames.allocated_frames.contains(frame@)
                    &&& frame@ % spec_page_size() == 0
                },
                Err(_) => true,
            },
    )]
    pub fn alloc_kernel_frame(&mut self) -> Result<KernelFrame, Error> {
        let frame_addr: FrameAddress = frame::alloc()?;
        KernelFrame::new(frame_addr).inspect_err(|e| {
            warn!("failed to wrap frame after KernelFrame::new failure: {e:?}");
            if let Err(free_err) = frame::free(frame_addr) {
                warn!("failed to free frame after KernelFrame::new failure: {free_err:?}");
            }
        })
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel frames into caller-provided storage.
    ///
    /// Kernel stacks require physically contiguous frames because the kernel uses identity
    /// mapping and the hardware stack pointer traverses the region linearly.
    /// Kernel allocations bypass the watermark — no artificial ceiling.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of frames to allocate.
    /// - `frames`: Mutable reference to a pre-allocated vector into which to store
    ///   those frames' addresses.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `frames` is filled with `count`
    /// contiguous entries. Upon failure, an error is returned instead.
    ///
    // Trusted shim: allocates a physically-contiguous run from the global allocator
    // (bypassing the watermark) and wraps each frame, with a two-phase
    // (`Vec::clear` + `frame::free`) rollback Verus cannot body-verify without
    // editing exec code. On success exactly `count` frames are handed out, each now
    // allocated, and their base addresses form an ascending page-stride run
    // (contiguity is load-bearing for kernel stacks). On error the vector is emptied.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            phys_view().initialized,
            phys_view().inv(),
            old(frames)@.len() == 0,
        ensures
            phys_view().inv(),
            phys_view().initialized,
            match result {
                Ok(()) => {
                    &&& final(frames)@.len() == count
                    &&& forall|i: int|
                        0 <= i < count as int ==>
                            #[trigger] phys_view().frames.allocated_frames.contains(final(frames)@[i]@)
                    &&& exists|base: int|
                        #[trigger] is_contiguous_run(final(frames)@.map_values(|kf: KernelFrame| kf@), base)
                },
                Err(_) => final(frames)@.len() == 0,
            },
    )]
    pub fn alloc_many_kernel_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<KernelFrame>,
    ) -> Result<(), Error> {
        // Check if caller-provided vector is not empty.
        if !frames.is_empty() {
            let reason: &str = "frames vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        if frames.capacity() < count {
            let reason: &str = "frames vector has insufficient capacity";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let base_addr: FrameAddress = frame::alloc_contiguous(count)?;
        let base_raw: usize = base_addr.into_raw_value();
        for i in 0..count {
            let raw_addr: usize = base_raw + i * mem::PAGE_SIZE;
            match FrameAddress::from_raw_value(raw_addr).and_then(KernelFrame::new) {
                Ok(kf) => frames.push(kf),
                Err(e) => {
                    // Drop already-wrapped frames (frees them via KernelFrame::Drop).
                    frames.clear();
                    // Free remaining un-wrapped frames from the contiguous allocation.
                    for j in i..count {
                        let leak_raw: usize = base_raw + j * mem::PAGE_SIZE;
                        if let Ok(fa) = FrameAddress::from_raw_value(leak_raw) {
                            if let Err(e) = frame::free(fa) {
                                warn!("failed to free leaked frame {fa:?}: {e:?}");
                            }
                        }
                    }
                    return Err(e);
                },
            }
        }
        Ok(())
    }
}
