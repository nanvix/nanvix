// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Fixed-Size Bump Allocator
//!
//! A generic, `no_std`-compatible bump allocator that hands out fixed-size slots from a
//! statically reserved memory region (typically BSS). Each slot is allocated exactly once via an
//! atomic compare-and-swap index, making the allocator lock-free and safe for concurrent use.
//!
//! ## Key Types
//!
//! - [`BssStorage`] — unsafe trait implemented by the backing store provider.
//! - [`FixedSizeBumpAllocator`] — the allocator parameterised by unit size, alignment, and
//!   storage backend.
//! - [`BumpAllocError`] — error type returned when allocation fails.
//!
//! ## Usage
//!
//! 1. Define a static storage region and implement [`BssStorage`] for it.
//! 2. Create a `static` [`FixedSizeBumpAllocator`] with [`FixedSizeBumpAllocator::new()`].
//! 3. Call [`alloc()`](FixedSizeBumpAllocator::alloc) to obtain raw byte slots, or
//!    [`alloc_as()`](FixedSizeBumpAllocator::alloc_as) for typed `MaybeUninit<T>` slots.
//!
//! ## Example
//!
//! ```rust
//! use bump_allocator::{align_up, BssStorage, BumpAllocError, FixedSizeBumpAllocator};
//!
//! const UNIT_SIZE: usize = 64;
//! const UNIT_ALIGN: usize = 64;
//! const NUM_SLOTS: usize = 4;
//! const STRIDE: usize = match align_up(UNIT_SIZE, UNIT_ALIGN) {
//!     Some(v) => v,
//!     None => panic!("stride overflow"),
//! };
//! const STORAGE_SIZE: usize = NUM_SLOTS * STRIDE;
//!
//! #[repr(align(64))]
//! struct MyStorage {
//!     bytes: [u8; STORAGE_SIZE],
//! }
//!
//! static mut BACKING: MyStorage = MyStorage {
//!     bytes: [0; STORAGE_SIZE],
//! };
//!
//! struct MyBackend;
//!
//! // SAFETY: BACKING is a stable, properly aligned static region exclusively managed by
//! // the allocator.
//! unsafe impl BssStorage for MyBackend {
//!     const NUM_UNITS: usize = NUM_SLOTS;
//!     const STORAGE_SIZE: usize = STORAGE_SIZE;
//!
//!     fn as_mut_ptr() -> *mut u8 {
//!         unsafe { core::ptr::addr_of_mut!(BACKING.bytes) as *mut u8 }
//!     }
//! }
//!
//! // SAFETY: single allocator instance for `MyBackend`.
//! static ALLOCATOR: FixedSizeBumpAllocator<UNIT_SIZE, UNIT_ALIGN, MyBackend> =
//!     unsafe { FixedSizeBumpAllocator::new() };
//!
//! // Allocate raw byte slots.
//! let slot_a: &mut [u8; UNIT_SIZE] = ALLOCATOR.alloc().expect("slot a");
//! let slot_b: &mut [u8; UNIT_SIZE] = ALLOCATOR.alloc().expect("slot b");
//! assert_ne!(slot_a.as_ptr(), slot_b.as_ptr());
//!
//! // Allocate a typed slot (caller must initialise before reading).
//! let uninit: &mut core::mem::MaybeUninit<[u32; 16]> =
//!     unsafe { ALLOCATOR.alloc_as::<[u32; 16]>().expect("typed slot") };
//! uninit.write([0u32; 16]);
//! let typed: &mut [u32; 16] = unsafe { uninit.assume_init_mut() };
//! typed[0] = 42;
//! assert_eq!(typed[0], 42);
//! ```

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(any(test, feature = "std")), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    fmt,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::atomic::{
        AtomicUsize,
        Ordering,
    },
};
use ::vstd::prelude::*;

// Include specifications.
#[cfg(verus_keep_ghost)]
include!("lib.spec.rs");

// Include proofs.
#[cfg(verus_keep_ghost)]
include!("lib.proof.rs");

//==================================================================================================
// Traits and Types
//==================================================================================================

///
/// # Description
///
/// Aligns `value` up to the next multiple of `alignment`.
///
/// # Parameters
///
/// - `value`: Value to align.
/// - `alignment`: Alignment boundary.
///
/// # Returns
///
/// Returns the aligned value, or `None` if `alignment` is zero or the computation overflows.
///
#[verus_spec(result =>
    ensures
        match result {
            Some(r) => align_up_spec(value as nat, alignment as nat) == Some(r as nat),
            None => align_up_spec(value as nat, alignment as nat) is None,
        },
)]
pub const fn align_up(value: usize, alignment: usize) -> Option<usize> {
    if alignment == 0 {
        return None;
    }
    value.div_ceil(alignment).checked_mul(alignment)
}

///
/// # Description
///
/// Error type returned by [`FixedSizeBumpAllocator`] operations.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpAllocError {
    /// Storage capacity exhausted.
    Exhausted,
    /// Type size does not match allocator unit size.
    SizeMismatch,
    /// Type alignment exceeds allocator alignment.
    AlignmentMismatch,
    /// Internal arithmetic overflow.
    Overflow,
    /// Computed slot exceeds storage bounds.
    OutOfBounds,
    /// Computed slot is not properly aligned.
    Misaligned,
}

impl fmt::Display for BumpAllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted => write!(f, "fixed-size BSS allocator exhausted"),
            Self::SizeMismatch => write!(f, "type size does not match allocator unit size"),
            Self::AlignmentMismatch => write!(f, "type alignment exceeds allocator alignment"),
            Self::Overflow => write!(f, "allocator internal arithmetic overflow"),
            Self::OutOfBounds => write!(f, "allocator slot exceeds storage bounds"),
            Self::Misaligned => write!(f, "allocator returned misaligned slot"),
        }
    }
}

///
/// # Description
///
/// Backend trait for fixed-size allocators that draw memory from static storage.
///
/// # Safety
///
/// Implementers must guarantee that:
///
/// - `as_mut_ptr()` returns a stable pointer to a backing region of at least `STORAGE_SIZE`
///   bytes for the allocator lifetime.
/// - the backing region is writable and properly aligned for any allocation site that uses this
///   backend.
/// - the backing region is exclusively managed by this allocator API, so creating `&'static mut`
///   references from slots cannot alias with other mutable references.
///
pub unsafe trait BssStorage {
    /// Number of fixed-size units that can be allocated.
    const NUM_UNITS: usize;

    /// Total size in bytes of the backing storage region.
    const STORAGE_SIZE: usize;

    /// Returns a mutable pointer to the beginning of the storage region.
    #[verus_spec]
    fn as_mut_ptr() -> *mut u8;
}

///
/// # Description
///
/// Generic fixed-size bump allocator over statically reserved memory.
///
/// # Type Parameters
///
/// - `N`: Unit size in bytes.
/// - `A`: Unit alignment in bytes.
/// - `S`: Backing storage provider.
///
pub struct FixedSizeBumpAllocator<const N: usize, const A: usize, S: BssStorage> {
    /// Atomic bump index for the next available slot.
    next_slot: AtomicUsize,
    /// Marker for the backend storage provider.
    _storage: PhantomData<S>,
}

impl<const N: usize, const A: usize, S: BssStorage> FixedSizeBumpAllocator<N, A, S> {
    ///
    /// # Description
    ///
    /// Creates a new fixed-size bump allocator.
    ///
    /// # Returns
    ///
    /// Returns a new allocator.
    ///
    /// # Safety
    ///
    /// The caller must ensure that only **one** `FixedSizeBumpAllocator` instance exists for a
    /// given `S: BssStorage` backend at any time. Creating multiple allocators over the same
    /// backend causes independent bump counters, which leads to overlapping slot reservations
    /// and undefined behavior (multiple `&'static mut` references to the same memory).
    ///
    pub const unsafe fn new() -> Self {
        Self {
            next_slot: AtomicUsize::new(0),
            _storage: PhantomData,
        }
    }

    ///
    /// # Description
    ///
    /// Allocates the next fixed-size slot as a raw byte array.
    ///
    /// # Returns
    ///
    /// On success, returns a mutable reference to a unit-sized byte array.
    ///
    /// # Errors
    ///
    /// - [`BumpAllocError::Exhausted`] if storage capacity is exhausted.
    /// - [`BumpAllocError::Overflow`] if internal arithmetic overflows.
    /// - [`BumpAllocError::OutOfBounds`] if computed slot exceeds storage bounds.
    /// - [`BumpAllocError::Misaligned`] if computed slot is not properly aligned.
    ///
    pub fn alloc(&self) -> Result<&'static mut [u8; N], BumpAllocError> {
        // Reserve a slot index via compare-and-swap to avoid overshooting the counter.
        let mut idx: usize = 0;
        loop {
            let current: usize = self.next_slot.load(Ordering::Acquire);
            if current >= S::NUM_UNITS {
                return Err(BumpAllocError::Exhausted);
            }
            let next: usize = current.checked_add(1).ok_or(BumpAllocError::Overflow)?;
            if self
                .next_slot
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                idx = current;
                break;
            }
        }

        let stride: usize = align_up(N, A).ok_or(BumpAllocError::Overflow)?;
        let offset: usize = idx.checked_mul(stride).ok_or(BumpAllocError::Overflow)?;

        let base: usize = S::as_mut_ptr() as usize;
        let ptr: usize = base.checked_add(offset).ok_or(BumpAllocError::Overflow)?;
        let end: usize = ptr.checked_add(N).ok_or(BumpAllocError::Overflow)?;
        let storage_end: usize = base
            .checked_add(S::STORAGE_SIZE)
            .ok_or(BumpAllocError::Overflow)?;
        if end > storage_end {
            return Err(BumpAllocError::OutOfBounds);
        }
        if !ptr.is_multiple_of(A) {
            return Err(BumpAllocError::Misaligned);
        }

        // SAFETY: the backend contract guarantees valid stable storage; each slot index is handed
        // out at most once by atomic compare-and-swap.
        Ok(unsafe { &mut *(ptr as *mut [u8; N]) })
    }

    ///
    /// # Description
    ///
    /// Allocates the next fixed-size slot and reinterprets it as `MaybeUninit<T>`.
    ///
    /// # Returns
    ///
    /// On success, returns a mutable reference to an uninitialized `T`.
    ///
    /// # Errors
    ///
    /// - [`BumpAllocError::SizeMismatch`] if `size_of::<T>() != N`.
    /// - [`BumpAllocError::AlignmentMismatch`] if `align_of::<T>() > A`.
    /// - Any error from [`alloc()`](Self::alloc).
    ///
    /// # Safety
    ///
    /// The caller must initialise the returned `MaybeUninit<T>` before reading through it
    /// and ensure exclusive use of the returned reference.
    ///
    pub unsafe fn alloc_as<T>(&self) -> Result<&'static mut MaybeUninit<T>, BumpAllocError> {
        if core::mem::size_of::<T>() != N {
            return Err(BumpAllocError::SizeMismatch);
        }
        if core::mem::align_of::<T>() > A {
            return Err(BumpAllocError::AlignmentMismatch);
        }

        let slot: &'static mut [u8; N] = self.alloc()?;
        // SAFETY: size/alignment checks above guarantee this cast is valid.
        Ok(unsafe { &mut *(slot.as_mut_ptr() as *mut MaybeUninit<T>) })
    }
}

impl<const N: usize, const A: usize, S: BssStorage> Default for FixedSizeBumpAllocator<N, A, S> {
    fn default() -> Self {
        // SAFETY: caller is responsible for the singleton invariant.
        unsafe { Self::new() }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{
        BssStorage,
        BumpAllocError,
        FixedSizeBumpAllocator,
    };
    use std::vec::Vec;

    #[repr(align(8))]
    struct StorageA {
        bytes: [u8; 16],
    }

    static mut STORAGE_A: StorageA = StorageA { bytes: [0; 16] };

    struct BackendA;

    unsafe impl BssStorage for BackendA {
        const NUM_UNITS: usize = 2;
        const STORAGE_SIZE: usize = 16;

        fn as_mut_ptr() -> *mut u8 {
            unsafe { core::ptr::addr_of_mut!(STORAGE_A.bytes) as *mut u8 }
        }
    }

    static ALLOC_A: FixedSizeBumpAllocator<8, 8, BackendA> =
        unsafe { FixedSizeBumpAllocator::new() };

    #[test]
    fn alloc_returns_distinct_slots() {
        let first: *mut u8 = ALLOC_A.alloc().expect("first alloc failed").as_mut_ptr();
        let second: *mut u8 = ALLOC_A.alloc().expect("second alloc failed").as_mut_ptr();
        assert_ne!(first, second);
    }

    #[repr(align(4))]
    struct StorageB {
        bytes: [u8; 8],
    }

    static mut STORAGE_B: StorageB = StorageB { bytes: [0; 8] };

    struct BackendB;

    unsafe impl BssStorage for BackendB {
        const NUM_UNITS: usize = 1;
        const STORAGE_SIZE: usize = 8;

        fn as_mut_ptr() -> *mut u8 {
            unsafe { core::ptr::addr_of_mut!(STORAGE_B.bytes) as *mut u8 }
        }
    }

    static ALLOC_B: FixedSizeBumpAllocator<8, 4, BackendB> =
        unsafe { FixedSizeBumpAllocator::new() };

    #[test]
    fn alloc_as_allows_typed_access() {
        let slot: &mut core::mem::MaybeUninit<[u32; 2]> =
            unsafe { ALLOC_B.alloc_as::<[u32; 2]>().expect("alloc_as failed") };
        slot.write([7, 11]);
        let typed: &mut [u32; 2] = unsafe { slot.assume_init_mut() };
        let values: Vec<u32> = Vec::from(*typed);
        assert_eq!(values, vec![7, 11]);
    }

    #[repr(align(8))]
    struct StorageC {
        bytes: [u8; 8],
    }

    static mut STORAGE_C: StorageC = StorageC { bytes: [0; 8] };

    struct BackendC;

    unsafe impl BssStorage for BackendC {
        const NUM_UNITS: usize = 1;
        const STORAGE_SIZE: usize = 8;

        fn as_mut_ptr() -> *mut u8 {
            unsafe { core::ptr::addr_of_mut!(STORAGE_C.bytes) as *mut u8 }
        }
    }

    static ALLOC_C: FixedSizeBumpAllocator<8, 8, BackendC> =
        unsafe { FixedSizeBumpAllocator::new() };

    #[test]
    fn alloc_returns_exhausted_error() {
        let _ = ALLOC_C.alloc().expect("first alloc failed");
        let result = ALLOC_C.alloc();
        assert_eq!(result.unwrap_err(), BumpAllocError::Exhausted);
    }
}
