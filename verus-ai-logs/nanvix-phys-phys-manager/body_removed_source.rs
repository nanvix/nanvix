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
    pub(super) fn init(upool: Upool) -> Result<(), Error> { ... }
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
    pub unsafe fn get_mut<'a>() -> &'a mut PhysMemoryManager { ... }
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
    ) -> Result<(), Error> { ... }

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
    pub fn alloc_user_frame(&mut self) -> Result<UserFrame, Error> { ... }

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
    fn check_user_watermark(count: usize) -> Result<(), Error> { ... }

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
    pub fn alloc_kernel_frame(&mut self) -> Result<KernelFrame, Error> { ... }

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
    ) -> Result<(), Error> { ... }
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
fn kernel_watermark() -> usize { ... }
