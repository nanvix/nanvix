// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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

    ///
    /// # Description
    ///
    /// Allocates a physically contiguous region of kernel frames and returns the base frame
    /// address, without any per-frame bookkeeping vector.
    ///
    /// This is intended for transient kernel-side staging buffers (for example, the `execv()` path
    /// stages the argument/environment strings read from user space into such a region). Because the
    /// microvm platform identity-maps physical memory into the
    /// kernel, the returned base doubles as a kernel-readable/writable pointer to a
    /// `count * PAGE_SIZE` byte region. The caller is responsible for releasing the region with
    /// [`Self::free_kernel_region`].
    ///
    /// Kernel allocations bypass the user watermark — no artificial ceiling.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of contiguous frames to allocate.
    ///
    /// # Returns
    ///
    /// Upon success, the base [`FrameAddress`] of the contiguous range is returned. Upon failure,
    /// an error is returned instead.
    ///
    pub fn alloc_kernel_region(&mut self, count: usize) -> Result<FrameAddress, Error> {
        if count == 0 {
            let reason: &str = "zero-length kernel region";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        frame::alloc_contiguous(count)
    }

    ///
    /// # Description
    ///
    /// Releases a contiguous region of kernel frames previously obtained from
    /// [`Self::alloc_kernel_region`].
    ///
    /// # Parameters
    ///
    /// - `base`: Base frame address of the region.
    /// - `count`: Number of contiguous frames in the region.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead; any frames
    /// that could be released are released regardless.
    ///
    pub fn free_kernel_region(&mut self, base: FrameAddress, count: usize) -> Result<(), Error> {
        if count == 0 {
            let reason: &str = "zero-length kernel region";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }
        let base_raw: usize = base.into_raw_value();
        let mut result: Result<(), Error> = Ok(());
        for i in 0..count {
            let raw_addr: usize = base_raw + i * mem::PAGE_SIZE;
            match FrameAddress::from_raw_value(raw_addr) {
                Ok(fa) => {
                    if let Err(e) = frame::free(fa) {
                        warn!("free_kernel_region(): failed to free frame {fa:?}: {e:?}");
                        result = Err(e);
                    }
                },
                Err(e) => {
                    warn!("free_kernel_region(): invalid frame address {raw_addr:#x}: {e:?}");
                    result = Err(e);
                },
            }
        }
        result
    }

    ///
    /// # Description
    ///
    /// Reclaims a previously booked (reserved) physical region, returning its frames to the
    /// allocator so they can satisfy later allocations. This is used to release the boot-modules
    /// region once every server has been copied into its own address space: the initrd images are
    /// then dead weight, and in debug builds they can dominate physical memory.
    ///
    /// The region is page-aligned (the base is rounded down and the end up) so that whole frames
    /// are released even when the booked payload begins at an offset within a page.
    ///
    /// # Parameters
    ///
    /// - `base_phys`: Physical base address of the booked region.
    /// - `size`: Size of the booked region in bytes.
    ///
    /// # Returns
    ///
    /// The number of frames returned to the allocator (zero when the region is empty or its base
    /// is invalid).
    ///
    pub fn reclaim_booked_region(&mut self, base_phys: usize, size: usize) -> usize {
        if size == 0 {
            return 0;
        }
        let page_start: usize = base_phys & !(mem::PAGE_SIZE - 1);
        let raw_end: usize = match base_phys.checked_add(size) {
            Some(end) => end,
            None => return 0,
        };
        let page_end: usize = match raw_end.checked_add(mem::PAGE_SIZE - 1) {
            Some(v) => v & !(mem::PAGE_SIZE - 1),
            None => return 0,
        };
        let count: usize = (page_end - page_start) / mem::PAGE_SIZE;
        match FrameAddress::from_raw_value(page_start) {
            // `free_kernel_region` frees what it can even on partial failure, so the region is
            // considered reclaimed regardless of its result.
            Ok(base) => {
                let _ = self.free_kernel_region(base, count);
                count
            },
            Err(e) => {
                warn!("reclaim_booked_region(): invalid base {page_start:#x}: {e:?}");
                0
            },
        }
    }
}
