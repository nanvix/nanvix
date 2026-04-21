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

//==================================================================================================
// Inner
//==================================================================================================

/// Private state of the kernel pool singleton.
struct Inner {
    /// Base address of the kernel pool.
    base: PageAligned<PhysicalAddress>,
    /// Bitmap of free frames.
    bitmap: Bitmap,
}

impl Inner {
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
    fn alloc(&mut self) -> Result<FrameAddress, Error> {
        let index: usize = match self.bitmap.alloc() {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?}");
                return Err(error);
            },
        };
        let addr: usize = self.base.into_raw_value() + index * mem::PAGE_SIZE;
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(addr)?)?))
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of frame addresses from the kernel pool.
    ///
    /// # Parameters
    ///
    /// - `addrs`: Mutable reference to a pre-allocated vector. The number of frames allocated
    ///   equals `addrs.capacity()`.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `addrs` is filled to capacity with contiguous
    /// entries. Upon failure, an error is returned instead.
    ///
    fn alloc_range(&mut self, addrs: &mut Vec<FrameAddress>) -> Result<(), Error> {
        if !addrs.is_empty() {
            let reason: &str = "addrs vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let count: usize = addrs.capacity();
        let index: usize = match self.bitmap.alloc_range(count) {
            Ok(index) => index,
            Err(error) => {
                error!("{error:?} (count={count})");
                return Err(error);
            },
        };

        let base_addr: usize = self.base.into_raw_value() + index * mem::PAGE_SIZE;
        for i in 0..count {
            let addr: usize = base_addr + i * mem::PAGE_SIZE;
            let frame: FrameAddress = FrameAddress::new(PageAligned::from_address(
                PhysicalAddress::from_raw_value(addr)?,
            )?);
            addrs.push(frame);
        }

        Ok(())
    }

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
    fn free(&mut self, addr: FrameAddress) -> Result<(), Error> {
        let index: usize = (addr.into_raw_value() - self.base.into_raw_value()) / mem::PAGE_SIZE;
        match self.bitmap.clear(index) {
            Ok(()) => Ok(()),
            Err(error) => {
                error!("{error:?} (addr={addr:?})");
                Err(error)
            },
        }
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
fn instance() -> &'static mut Inner {
    if unlikely(!INSTANCE_INIT.load(ORDER)) {
        panic!("kernel pool used before init()");
    }

    // SAFETY: `INSTANCE_INIT` is `true`, so `INSTANCE` has been fully written by `init()`.
    // The kernel is single-threaded with interrupts disabled, so no concurrent access is possible.
    unsafe { INSTANCE.assume_init_mut() }
}

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
) -> Result<Kpool, Error> {
    if unlikely(INSTANCE_INIT.load(ORDER)) {
        return Err(Error::new(ErrorCode::InvalidArgument, "kernel pool already initialized"));
    }

    trace!("base={base:?}");

    // Check if bitmap spans across physically-addressable memory.
    let bitmap_capacity: usize = bitmap.number_of_bits();
    let kpool_size: usize = bitmap_capacity * mem::PAGE_SIZE;
    if !is_valid_physical_region(base.into_raw_value(), kpool_size) {
        let reason: &str = "kernel pool bitmap spans across physically-addressable memory";
        error!("{reason}");
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    let num_frames: usize = bitmap.number_of_bits();
    info!("kernel pool: {} frames, {} KB", num_frames, (num_frames * mem::PAGE_SIZE) / 1024,);

    // SAFETY: single-threaded boot; no other reference to `INSTANCE` exists.
    unsafe { INSTANCE.write(Inner { base, bitmap }) };
    INSTANCE_INIT.store(true, ORDER);
    Ok(Kpool { _private: () })
}

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
fn alloc() -> Result<FrameAddress, Error> {
    instance().alloc()
}

///
/// # Description
///
/// Allocates a contiguous range of frames from the kernel pool.
///
/// # Parameters
///
/// - `addrs`: Mutable reference to a pre-allocated vector. The number of frames allocated
///   equals `addrs.capacity()`.
///
/// # Return Values
///
/// Upon success, `Ok(())` is returned and `addrs` is filled to capacity with contiguous
/// entries. Upon failure, an error is returned instead.
///
fn alloc_range(addrs: &mut Vec<FrameAddress>) -> Result<(), Error> {
    instance().alloc_range(addrs)
}

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
fn free(addr: FrameAddress) -> Result<(), Error> {
    instance().free(addr)
}

//==================================================================================================
// Kernel Frame
//==================================================================================================

/// A type that represents a kernel frame.
#[derive(Debug)]
pub struct KernelFrame {
    /// Frame address.
    base: FrameAddress,
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
    fn new(base: FrameAddress) -> Self {
        Self { base }
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the target kernel frame.
    ///
    /// # Returns
    ///
    /// The base address of the target kernel frame.
    ///
    pub fn base(&self) -> FrameAddress {
        self.base
    }

    ///
    /// # Description
    ///
    /// Clears the target kernel frame.
    ///
    pub fn clear(&mut self) {
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
        if let Err(e) = free(self.base) {
            error!("failed to free kernel frame: {:?}", e);
        }
    }
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
    pub fn alloc(&mut self) -> Result<KernelFrame, Error> {
        let addr: FrameAddress = alloc()?;
        Ok(KernelFrame::new(addr))
    }

    ///
    /// # Description
    ///
    /// Allocates a contiguous range of kernel frames from the kernel frame pool.
    ///
    /// # Parameters
    ///
    /// - `frames`: Mutable reference to a pre-allocated vector. The number of frames allocated
    ///   equals `frames.capacity()`.
    ///
    /// # Return Values
    ///
    /// Upon success, `Ok(())` is returned and `frames` is filled to capacity with contiguous
    /// entries. Upon failure, an error is returned instead.
    ///
    pub fn alloc_many(&mut self, frames: &mut Vec<KernelFrame>) -> Result<(), Error> {
        // Check if caller-provided vector is not empty.
        if !frames.is_empty() {
            let reason: &str = "frames vector is not empty";
            error!("{reason}");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let count: usize = frames.capacity();
        let mut addrs: Vec<FrameAddress> = Vec::with_capacity(count);
        alloc_range(&mut addrs)?;
        for addr in addrs {
            frames.push(KernelFrame::new(addr));
        }
        Ok(())
    }
}
