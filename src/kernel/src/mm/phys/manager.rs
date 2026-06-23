// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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

#[verus_verify]
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
    // TCB: bringing up the manager singleton writes a `MaybeUninit` static behind an
    // `AtomicBool` flag — raw-memory/atomics operations outside Verus's model. On success the
    // manager layer becomes ready (`phys_view().manager_ready`); the frame partition is
    // untouched. Listed in `verus-ai-logs/tcb-allowed.md`.
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        ensures
            match result {
                Ok(_) => crate::mm::phys::phys_view().manager_ready,
                Err(_) => crate::mm::phys::phys_view().manager_ready,
            },
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
}

impl PhysMemoryManager {
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
}

#[verus_verify]
impl PhysMemoryManager {
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
    #[verus_spec(result =>
        requires
            self.inv(),
            old(frames)@.len() == 0,
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& (count > 0 ==> old(self)@.user_alloc_ok(count as nat))
                    &&& final(frames)@.len() == count
                    &&& user_addr_set(final(frames)@).len() == count
                    &&& old(self)@.all_free(user_addr_set(final(frames)@))
                    &&& final(self)@ == old(self)@.book_all(user_addr_set(final(frames)@))
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& final(frames)@.len() == 0
                },
            },
    )]
    pub fn alloc_many_user_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<UserFrame>,
    ) -> Result<(), Error> {
        proof_decl! {
            let ghost g_old = self@;
        }
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
            proof! {
                lemma_user_addr_set_empty(frames@);
                lemma_book_all_empty(g_old);
            }
            return Ok(());
        }

        Self::check_user_watermark(count)?;
        proof! {
            lemma_user_bulk_start(self, g_old, frames@, count as nat);
        }

        #[verus_spec(
            invariant
                g_old == old(self)@,
                g_old.wf(),
                self@.wf(),
                frames@.len() == _idx,
                user_bulk_inv(g_old, self@, frames@),
                count > 0 ==> g_old.user_alloc_ok(count as nat),
        )]
        for _idx in 0..count {
            proof_decl! {
                let ghost mview_pre = self@;
            }
            match self.upool.alloc() {
                Ok(frame) => {
                    proof! {
                        lemma_user_bulk_step(g_old, mview_pre, frames@, frame);
                    }
                    frames.push(frame);
                },
                Err(error) => {
                    frames.clear();
                    proof! {
                        lemma_user_bulk_err_restored(self, g_old);
                    }
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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(uf) => {
                    &&& old(self)@.user_alloc_ok(1)
                    &&& old(self)@.is_free(uf@)
                    &&& final(self)@ == old(self)@.alloc_one(uf@)
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !old(self)@.user_alloc_ok(1)
                },
            },
    )]
    pub fn alloc_user_frame(&mut self) -> Result<UserFrame, Error> {
        proof! {
            lemma_manager_attached(self);
        }
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
    #[verus_spec(result =>
        ensures
            match result {
                Ok(()) => crate::mm::phys::phys_view().frames.free_count()
                    >= count as nat + spec_kernel_watermark(),
                Err(_) => crate::mm::phys::phys_view().frames.free_count()
                    < count as nat + spec_kernel_watermark(),
            },
    )]
    fn check_user_watermark(count: usize) -> Result<(), Error> {
        // Reading the free count first pins the abstract `free_count()` to a concrete `usize`,
        // hence `free_count() <= usize::MAX`. This bound is what the overflow arm below relies on
        // (a `kernel_watermark() + count` overflow means the threshold exceeds any possible free
        // count). Binding it here makes that fact available on every path without an axiom.
        let available: usize = frame::free_count();
        let watermark_threshold: usize =
            kernel_watermark().checked_add(count).ok_or_else(|| {
                let reason: &str = "watermark + count overflow";
                error!("{reason}");
                Error::new(ErrorCode::InvalidArgument, reason)
            })?;
        if available < watermark_threshold {
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
    #[verus_spec(result =>
        requires
            self.inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(kf) => {
                    &&& old(self)@.is_free(kf@)
                    &&& final(self)@ == old(self)@.alloc_one(kf@)
                },
                Err(_) => final(self)@ == old(self)@,
            },
    )]
    pub fn alloc_kernel_frame(&mut self) -> Result<KernelFrame, Error> {
        proof_decl! {
            let ghost g_old = self@;
        }
        let frame_addr: FrameAddress = frame::alloc()?;
        let result: Result<KernelFrame, Error> = KernelFrame::new(frame_addr).inspect_err(|e| {
            warn!("failed to wrap frame after KernelFrame::new failure: {e:?}");
            if let Err(free_err) = frame::free(frame_addr) {
                warn!("failed to free frame after KernelFrame::new failure: {free_err:?}");
            }
        });
        proof! {
            if result is Ok {
                lemma_kernel_alloc_one(g_old, self@, result->Ok_0@);
            }
        }
        result
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
    #[verus_spec(result =>
        requires
            self.inv(),
            old(frames)@.len() == 0,
            count > 0,
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    &&& final(frames)@.len() == count
                    &&& kernel_frames_contiguous(final(frames)@, count as nat)
                    &&& old(self)@.all_free(kernel_addr_set(final(frames)@))
                    &&& final(self)@ == old(self)@.book_all(kernel_addr_set(final(frames)@))
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& final(frames)@.len() == 0
                },
            },
    )]
    pub fn alloc_many_kernel_frames(
        &mut self,
        count: usize,
        frames: &mut Vec<KernelFrame>,
    ) -> Result<(), Error> {
        proof_decl! {
            let ghost g_old = self@;
        }
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
        #[verus_spec(
            invariant
                g_old == old(self)@,
                g_old.wf(),
                self@ == g_old,
                base_raw as int == base_addr@,
                base_raw as int + (count as int) * spec_page_size() <= usize::MAX as int,
        )]
        for i in 0..count {
            proof! {
                lemma_contig_no_overflow(base_raw, i, count);
            }
            let raw_addr: usize = base_raw + i * mem::PAGE_SIZE;
            match FrameAddress::from_raw_value(raw_addr).and_then(KernelFrame::new) {
                Ok(kf) => frames.push(kf),
                Err(e) => {
                    // Drop already-wrapped frames (frees them via KernelFrame::Drop).
                    frames.clear();
                    // Free remaining un-wrapped frames from the contiguous allocation.
                    #[verus_spec(
                        invariant
                            g_old == old(self)@,
                            g_old.wf(),
                            self@ == g_old,
                            base_raw as int == base_addr@,
                            base_raw as int + (count as int) * spec_page_size()
                                <= usize::MAX as int,
                    )]
                    for j in i..count {
                        proof! {
                            lemma_contig_no_overflow(base_raw, j, count);
                        }
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
        proof! {
            lemma_kernel_alloc_contiguous(g_old, self@, frames@, count as nat);
        }
        Ok(())
    }
}

impl PhysMemoryManager {
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
}

//==================================================================================================
// Build-time constant accessors
//==================================================================================================

// External-bottom trust boundary: `config::kernel::KERNEL_WATERMARK` is generated by the `config`
// crate's `build.rs` (from `kernel_config.toml`) and lives in a non-Verus dependency crate, so
// Verus cannot resolve its value. This accessor ties the runtime constant to the abstract
// `spec_kernel_watermark()`. Listed in `verus-ai-logs/tcb-allowed.md`.
#[verus_verify(external_body)]
#[verus_spec(ret =>
    ensures
        ret as nat == spec_kernel_watermark(),
)]
fn kernel_watermark() -> usize {
    config::kernel::KERNEL_WATERMARK
}
