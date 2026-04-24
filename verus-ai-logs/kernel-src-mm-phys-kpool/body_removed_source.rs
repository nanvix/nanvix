// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Kernel frame pool — module-level singleton.
//!
//! The kernel pool is backed by a [`Bitmap`] and exposed as free functions over a singleton so
//! every in-kernel caller goes through the same state. The public facade types [`Kpool`] and
//! [`KernelFrame`] delegate to the singleton.
//!
//! Access to the kernel pool is synchronized externally and performed by a single thread, so
//! the backing bitmap uses non-atomic operations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    collections::Bitmap,
    hal::{
        mem::{
            Address,
            FrameAddress,
            PageAligned,
            PhysicalAddress,
        },
        platform::is_valid_physical_region,
    },
};
use ::alloc::vec::Vec;
use ::arch::mem;
use ::core::{
    hint::unlikely,
    mem::MaybeUninit,
    ops::{
        Deref,
        DerefMut,
    },
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

#[cfg(verus_keep_ghost)]
include!("kpool.spec.rs");

#[cfg(verus_keep_ghost)]
include!("kpool.proof.rs");

//==================================================================================================
// Inner
//==================================================================================================

/// Private state of the kernel pool singleton.
#[verus_verify(external_derive)]
struct Inner {
    /// Base address of the kernel pool.
    base: PageAligned<PhysicalAddress>,
    /// Bitmap of free frames.
    bitmap: Bitmap,
}

#[verus_verify]
impl Inner {
    ///
    /// # Description
    ///
    /// Creates a new kernel pool.
    ///
    /// # Return Values
    ///
    /// Upon success, the kernel pool.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            base.inv(),
            bitmap.inv(),
        ensures
            result matches Ok(kpool) ==> {
                &&& kpool.inv()
                &&& kpool@.start == base@
                &&& kpool@.num_pages == bitmap@.num_bits
                &&& kpool@.used_page_indices == Set::<int>::new(|i: int| bitmap@.is_bit_set(i))
            },
    )]
    fn new(base: PageAligned<PhysicalAddress>, bitmap: Bitmap) -> Result<Inner, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates a frame from the kernel pool.
    ///
    /// # Return Values
    ///
    /// Upon success, the address of the allocated frame is returned. Upon failure, an error is
    /// returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(frame) => {
                    let page_index = (frame@ - old(self)@.start) / spec_page_size();
                    &&& frame.inv()
                    &&& 0 <= page_index < old(self)@.num_pages
                    &&& !old(self)@.used_page_indices.contains(page_index)
                    &&& self@ == KpoolView {
                        used_page_indices: old(self)@.used_page_indices.insert(page_index),
                        ..old(self)@
                    }
                },
                Err(_) => {
                    &&& forall|i: int| 0 <= i < old(self)@.num_pages ==> old(self)@.used_page_indices.contains(i)
                    &&& self@ == old(self)@
                },
            },
    )]
    fn alloc(&mut self) -> Result<FrameAddress, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of frame addresses from the kernel pool.
    ///
    /// # Parameters
    ///
    /// - `count` - The number of frames to allocate.
    /// - `addrs`: Mutable reference to a pre-allocated vector in which
    ///   to store those frames' addresses.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `addrs` is filled with `count`
    /// contiguous entries. Upon failure, an error is returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            self.inv(),
            match result {
                Ok(()) => {
                    &&& old(addrs)@.len() == 0
                    &&& count > 0
                    &&& addrs@.len() == count
                    &&& forall|which_frame: int| #![trigger addrs@[which_frame]]
                        0 <= which_frame < count ==> {
                            let frame = addrs@[which_frame];
                            let addr = frame@;
                            let page_index = (addr - old(self)@.start) / spec_page_size();
                            &&& frame.inv()
                            &&& 0 <= page_index < old(self)@.num_pages
                            &&& addr == addrs@[0]@ + which_frame * spec_page_size()
                            &&& !old(self)@.used_page_indices.contains(page_index)
                        }
                    &&& {
                        let first_page_index = (addrs@[0]@ - old(self)@.start) / spec_page_size();
                        let new_page_indices = Set::<int>::new(
                            |i: int| first_page_index <= i < first_page_index + count
                        );
                        self@ == KpoolView {
                            used_page_indices: old(self)@.used_page_indices.union(new_page_indices),
                            ..old(self)@
                        }
                    }
                },
                Err(_) => {
                    &&& count == 0 || forall|i: int| !old(self)@.range_free(i, count as int)
                    &&& self@ == old(self)@
                    &&& addrs@ == old(addrs)@
                },
            },
    )]
    fn alloc_range(&mut self, count: usize, addrs: &mut Vec<FrameAddress>) -> Result<(), Error> { ... }

    ///
    /// # Description
    ///
    /// Frees a previously allocated frame in the kernel pool.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address of the frame to free.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self).inv(),
            addr.inv(),
        ensures
            self.inv(),
            ({
                let page_index = (addr@ - old(self)@.start) / spec_page_size();
                let input_valid = {
                    &&& 0 <= page_index < old(self)@.num_pages
                    &&& old(self)@.used_page_indices.contains(page_index)
                };
                match result {
                    Ok(()) => {
                        &&& input_valid
                        &&& self@ == KpoolView {
                              used_page_indices: old(self)@.used_page_indices.remove(page_index),
                              ..old(self)@
                        }
                    },
                    Err(_) => {
                        &&& !input_valid
                        &&& self@ == old(self)@
                    },
                }
            }),
    )]
    fn free(&mut self, addr: FrameAddress) -> Result<(), Error> { ... }
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

/// Whether the kernel pool has been initialized.
static INSTANCE_INIT: AtomicBool = AtomicBool::new(false);

///
/// # Description
///
/// Returns a mutable reference to the initialized singleton.
///
/// # Return Values
///
/// A mutable reference to the kernel pool singleton.
///
fn instance() -> &'static mut Inner { ... }

//==================================================================================================
// Public Free Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the kernel pool singleton.
///
/// # Parameters
///
/// - `base`: Base address of the kernel pool.
/// - `bitmap`: Bitmap for tracking free pages.
///
/// # Return Values
///
/// Upon success, a [`Kpool`] instance is returned. Upon failure, an error is returned instead.
///
/// # Safety
///
/// Must be called exactly once during boot, before any other function in this module.
///
pub(super) unsafe fn init(
    base: PageAligned<PhysicalAddress>,
    bitmap: Bitmap,
) -> Result<Kpool, Error> { ... }

///
/// # Description
///
/// Allocates a frame from the kernel pool.
///
/// # Return Values
///
/// Upon success, the address of the allocated frame is returned. Upon failure, an error is
/// returned instead.
///
fn alloc() -> Result<FrameAddress, Error> { ... }

///
/// # Description
///
/// Allocates a contiguous range of frames from the kernel pool.
///
/// # Parameters
///
/// - `count`: Number of frames to allocate.
/// - `addrs`: Mutable reference to a pre-allocated vector into which to
///   store those frames' addresses.
///
/// # Return Values
///
/// Upon success, `Ok(())` is returned and `addrs` is filled with `count`
/// contiguous entries. Upon failure, an error is returned instead.
///
fn alloc_range(count: usize, addrs: &mut Vec<FrameAddress>) -> Result<(), Error> { ... }

///
/// # Description
///
/// Frees a frame previously returned by [`alloc`].
///
/// # Parameters
///
/// - `addr`: Address of the frame to free.
///
/// # Return Values
///
/// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
///
fn free(addr: FrameAddress) -> Result<(), Error> { ... }

//==================================================================================================
// Kernel Frame
//==================================================================================================

/// A type that represents a kernel frame.
#[verus_verify(external_derive)]
#[derive(Debug)]
pub struct KernelFrame {
    /// Frame address.
    base: FrameAddress,
}

#[cfg(verus_keep_ghost)]
verus! {

use crate::hal::mem::spec_page_size;

impl View for KernelFrame
{
    type V = int;

    closed spec fn view(&self) -> int
    {
        self.base@
    }
}

}

impl KernelFrame {
    ///
    /// # Description
    ///
    /// Instantiates a kernel frame.
    ///
    /// # Parameters
    ///
    /// - `base`: Frame address.
    ///
    /// # Returns
    ///
    /// A kernel frame.
    ///
    fn new(base: FrameAddress) -> Self { ... }

    ///
    /// # Description
    ///
    /// Returns the base address of the target kernel frame.
    ///
    /// # Returns
    ///
    /// The base address of the target kernel frame.
    ///
    pub fn base(&self) -> FrameAddress { ... }

    ///
    /// # Description
    ///
    /// Clears the target kernel frame.
    ///
    pub fn clear(&mut self) { ... }
}

impl Deref for KernelFrame {
    type Target = [u8];

    fn deref(&self) -> &Self::Target { ... }
}

impl DerefMut for KernelFrame {
    fn deref_mut(&mut self) -> &mut Self::Target { ... }
}

impl Drop for KernelFrame {
    fn drop(&mut self) { ... }
}

//==================================================================================================
// Kernel Pool
//==================================================================================================

///
/// # Description
///
/// Thin facade over the module-level kernel pool singleton. Exists as a distinct type so
/// kernel-frame allocation has its own entry point ([`Kpool::alloc`] returning [`KernelFrame`]).
///
#[derive(Debug)]
pub struct Kpool {
    /// Private field prevents external construction.
    _private: (),
}

impl Kpool {
    ///
    /// # Description
    ///
    /// Allocates a kernel frame from the kernel frame pool.
    ///
    /// # Return Values
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    pub fn alloc(&mut self) -> Result<KernelFrame, Error> { ... }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel frames from the kernel frame pool.
    ///
    /// # Parameters
    ///
    /// - `count`: Number of frames to allocate.
    /// - `frames`: Mutable reference to a pre-allocated vector into which
    ///   to store those frames' addresses. It must be pre-allocated with
    ///   capacity of at least `count`.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `frames` is filled with `count`
    /// contiguous entries. Upon failure, an error is returned instead.
    ///
    pub fn alloc_many(&mut self, count: usize, frames: &mut Vec<KernelFrame>) -> Result<(), Error> { ... }
}
