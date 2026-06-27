// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Verus
//==================================================================================================

use ::vstd::prelude::*;

#[cfg(verus_keep_ghost)]
include!("kheap.spec.rs");
#[cfg(verus_keep_ghost)]
include!("kheap.proof.rs");

//==================================================================================================
// Imports
//==================================================================================================

use crate::collections::Slab;
use ::alloc::alloc::{
    AllocError,
    GlobalAlloc,
    Layout,
};
use ::arch::mem;
use ::config::constants;
use ::core::ptr;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::type_safe::usize_to_mut_ptr;

//==================================================================================================
// Constants
//==================================================================================================

verus! {
/// Number of slabs in the heap. Each slab is responsible for allocating blocks of a specific size.
const NUM_OF_SLABS: usize = 7;
/// Number of slabs per slab size. Each slab size is allocated a fixed number of slabs.
pub(super) const SLAB_COUNT: usize = 32;
/// Minimum size of a single slab in bytes. It is calculated as the number of slabs per slab size
/// multiplied by the page size.
const MIN_SLAB_SIZE: usize = SLAB_COUNT * mem::PAGE_SIZE;
/// Minimum heap size in bytes. This is the minimum size of the backing storage that must be
/// provided to initialize the heap.
pub(crate) closed const MIN_HEAP_SIZE: usize = NUM_OF_SLABS * MIN_SLAB_SIZE;
/// Maximum slab size in bytes. Allocations whose size or alignment exceed this are rejected.
///
/// This must mirror [`config::kernel::MAX_SLAB_SIZE`], the shared kernel-heap budget. It is
/// duplicated as the literal `512` here because Verus cannot evaluate cross-crate constants in
/// specification contexts (and, separately, does not yet support enum discriminants in const
/// expressions: https://github.com/verus-lang/verus/issues/1107). The `assert_eq!` below makes this
/// a genuine dependency on the `config` crate: the kernel fails to compile if the two diverge.
const MAX_SLAB_SIZE: usize = 512;
}

// Bind the Verus-visible literal above to the shared budget defined in the `config` crate. This
// turns the duplicated literal into a compile-time dependency: if `config::kernel::MAX_SLAB_SIZE`
// ever changes, this assertion forces the kernel-heap constant to be updated in lockstep.
::static_assert::assert_eq!(MAX_SLAB_SIZE == ::config::kernel::MAX_SLAB_SIZE);

//==================================================================================================
//  Structures
//==================================================================================================

struct ArenaAllocator;

#[verus_verify]
#[derive(Copy, Clone)]
pub(super) enum SlabSize {
    Slab8 = 8,
    Slab16 = 16,
    Slab32 = 32,
    Slab64 = 64,
    Slab128 = 128,
    Slab256 = 256,
    Slab512 = 512,
}

#[verus_verify]
struct Kheap {
    slab_8_bytes: Slab,
    slab_16_bytes: Slab,
    slab_32_bytes: Slab,
    slab_64_bytes: Slab,
    slab_128_bytes: Slab,
    slab_256_bytes: Slab,
    slab_512_bytes: Slab,
    // Ghost state — alloc_map is the ground truth for abstract KheapView.
    // Ghost<T> is zero-sized; it costs nothing at runtime.
    #[cfg(verus_keep_ghost_body)]
    alloc_map: Ghost<Map<int, nat>>,
}

//==================================================================================================
// Global Variables
//==================================================================================================

static mut HEAP: Option<Kheap> = None;

/// Pointer to the platform-provided heap backing buffer.
/// Every platform must set this via [`set_backing_storage()`] before calling [`init()`].
static mut BACKING_PTR: *mut u8 = core::ptr::null_mut();
/// Size of the backing storage in bytes.
static mut BACKING_SIZE: usize = 0;

#[global_allocator]
static mut ALLOCATOR: ArenaAllocator = ArenaAllocator;

//==================================================================================================
// Implementations
//==================================================================================================

// VERUS DEVIATION: `addr as *mut u8` is unsupported in verified bodies.
// The executable cast is isolated behind the specified `::type_safe::usize_to_mut_ptr`
// helper; its trusted spec lives in `kheap.spec.rs`.
#[verus_verify]
impl Kheap {
    /// Constructs a Kheap over the `[addr, addr + size)` region.
    ///
    /// # Success
    /// - `addr` is page-aligned and non-zero
    /// - `size` is a positive multiple of `MIN_HEAP_SIZE`
    /// - Every slab tier is freshly initialized with zero live allocations
    /// - The resulting `KheapView` is empty (no live allocations).
    ///
    /// # Failure
    /// Returns `InvalidArgument` on alignment or sizing violations.
    #[verus_spec(result =>
        ensures
            match result {
                Ok(kheap) => {
                    &&& kheap.inv()
                    &&& kheap@ == KheapView::new()
                },
                Err(e) => e.code == ErrorCode::InvalidArgument,
            },
    )]
    unsafe fn from_raw_parts(addr: usize, size: usize) -> Result<Kheap, Error> {
        // Check if start address is zero.
        if addr == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "null start address"));
        }

        // Check if the region wraps around.
        if addr.checked_add(size).is_none() {
            return Err(Error::new(ErrorCode::InvalidArgument, "address space overflow"));
        }

        // Check if size exceeds isize::MAX.
        if size > isize::MAX as usize {
            return Err(Error::new(ErrorCode::InvalidArgument, "size exceeds isize::MAX"));
        }

        // Check if start address is not page aligned.
        if !addr.is_multiple_of(mem::PAGE_SIZE) {
            return Err(Error::new(ErrorCode::InvalidArgument, "unaligned start address"));
        }

        // Check if size is less than minimum heap size.
        if size < MIN_HEAP_SIZE {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "heap size is less than minimum heap size",
            ));
        }

        // Check if size is not a multiple of the minimum heap size.
        if !size.is_multiple_of(MIN_HEAP_SIZE) {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "size is not a multiple of the minimum heap size",
            ));
        }

        let heap_start_addr: *mut u8 = usize_to_mut_ptr(addr);
        let slab_size: usize = size / NUM_OF_SLABS;

        // `size` is a multiple of `MIN_HEAP_SIZE = NUM_OF_SLABS * MIN_SLAB_SIZE`,
        // so it's also a multiple of `NUM_OF_SLABS`, giving `slab_size * NUM == size`.
        proof! {
            lemma_from_raw_parts_ptr_preconditions(addr, size, slab_size, heap_start_addr);
        }

        info!("heap size: {} MB", size / constants::MEGABYTE);
        info!("slab size: {} KB", slab_size / constants::KILOBYTE);
        proof_with! {alloc_map: Ghost(Map::<int, nat>::empty())};
        let kheap = Kheap {
            slab_8_bytes: Slab::from_raw_parts(
                heap_start_addr,
                slab_size,
                SlabSize::Slab8 as usize,
            )?,
            slab_16_bytes: Slab::from_raw_parts(
                heap_start_addr.add(slab_size),
                slab_size,
                SlabSize::Slab16 as usize,
            )?,
            slab_32_bytes: Slab::from_raw_parts(
                heap_start_addr.add(2 * slab_size),
                slab_size,
                SlabSize::Slab32 as usize,
            )?,
            slab_64_bytes: Slab::from_raw_parts(
                heap_start_addr.add(3 * slab_size),
                slab_size,
                SlabSize::Slab64 as usize,
            )?,
            slab_128_bytes: Slab::from_raw_parts(
                heap_start_addr.add(4 * slab_size),
                slab_size,
                SlabSize::Slab128 as usize,
            )?,
            slab_256_bytes: Slab::from_raw_parts(
                heap_start_addr.add(5 * slab_size),
                slab_size,
                SlabSize::Slab256 as usize,
            )?,
            slab_512_bytes: Slab::from_raw_parts(
                heap_start_addr.add(6 * slab_size),
                slab_size,
                SlabSize::Slab512 as usize,
            )?,
        };
        Ok(kheap)
    }

    /// Allocates a block that fits the requested `layout`.
    ///
    /// # Success
    /// - Returns a non-null, properly aligned pointer
    /// - The returned address was not previously live
    /// - The abstract view records the new size at the returned addr.
    ///
    /// # Failure
    /// Abstract state is unchanged.
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(ptr) => {
                    &&& ptr as usize != 0
                    &&& (ptr as int) % (spec_layout_align(layout) as int) == 0
                    &&& if spec_layout_size(layout) == 0 {
                        final(self)@ == old(self)@
                    } else {
                        &&& !old(self)@.allocations.dom().contains(ptr as int)
                        &&& final(self)@.allocations[ptr as int] == spec_layout_size(layout) as nat
                        &&& final(self)@ == old(self)@.spec_allocate(
                            ptr as int,
                            spec_layout_size(layout) as nat,
                        )
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !(old(self)@.allocations == Map::<int, nat>::empty())
                        || spec_layout_align(layout) > spec_layout_size(layout)
                        || spec_layout_size(layout) > MAX_SLAB_SIZE
                },
            },
    )]
    unsafe fn allocate(&mut self, layout: Layout) -> Result<*mut u8, AllocError> {
        // Zero-sized allocations must return a non-null, suitably-aligned pointer without
        // touching any slab. Use the alignment as a dangling pointer, as the standard library does.
        if layout.size() == 0 {
            let align: usize = layout.align();
            proof! {
                reveal(is_pow2);
                assert(align as int % align as int == 0) by (nonlinear_arith)
                    requires align > 0;
            }
            return Ok(usize_to_mut_ptr(align));
        }
        // Reject layouts where alignment exceeds size or the maximum slab tier.
        if layout.align() > layout.size() || layout.align() > MAX_SLAB_SIZE {
            return Err(AllocError);
        }
        // Reveal is_pow2 so Verus can deduce align ≥ 1, which is needed to
        // rule out size == 0 when layout_to_allocator returns Err.
        proof! { reveal(is_pow2); }
        let tier: SlabSize = Kheap::layout_to_allocator(&layout)?;
        let r: Result<*mut u8, AllocError> = match tier {
            SlabSize::Slab8 => self.slab_8_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab16 => self.slab_16_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab32 => self.slab_32_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab64 => self.slab_64_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab128 => self.slab_128_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab256 => self.slab_256_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab512 => self.slab_512_bytes.allocate().map_err(|_e| AllocError),
        };
        proof! {
            let size = spec_layout_size(layout);
            let align = spec_layout_align(layout);
            if let Ok(ptr) = r {
                *self.alloc_map = self.alloc_map@.insert(ptr as int, size as nat);
            }
            Kheap::lemma_allocate_result(old(self), self, tier, r, size, align);
        }
        r
    }

    /// Deallocates the block at `ptr`, which was allocated by `allocate` with `layout`.
    ///
    /// # Success
    /// Removes `ptr` from the abstract view.
    ///
    /// # Failure
    /// Abstract state is unchanged.
    #[verus_spec(result =>
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            match result {
                Ok(()) => {
                    if spec_layout_size(layout) == 0 {
                        final(self)@ == old(self)@
                    } else {
                        &&& old(self)@.allocations.dom().contains(ptr as int)
                        &&& final(self)@ == old(self)@.spec_deallocate(ptr as int)
                    }
                },
                Err(_) => {
                    &&& final(self)@ == old(self)@
                    &&& !old(self)@.allocations.dom().contains(ptr as int)
                        || old(self)@.allocations[ptr as int] != spec_layout_size(layout) as nat
                },
            },
    )]
    unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) -> Result<(), AllocError> {
        // Deallocation of a zero-sized layout is a no-op, matching the allocation path above.
        if layout.size() == 0 {
            return Ok(());
        }
        proof! {
            let size = spec_layout_size(layout);
            if size == 0 || size > MAX_SLAB_SIZE {
                lemma_dealloc_layout_to_allocator_err(self, ptr as usize, size);
            }
        }
        let tier: SlabSize = Kheap::layout_to_allocator(&layout)?;
        let r: Result<(), AllocError> = match tier {
            SlabSize::Slab8 => self.slab_8_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab16 => self.slab_16_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab32 => self.slab_32_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab64 => self.slab_64_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab128 => self.slab_128_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab256 => self.slab_256_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab512 => self.slab_512_bytes.deallocate(ptr).map_err(|_e| AllocError),
        };
        proof! {
            let size = spec_layout_size(layout);
            if let Ok(()) = r {
                *self.alloc_map = self.alloc_map@.remove(ptr as int);
            }
            Kheap::lemma_deallocate_result(old(self), self, tier, ptr as usize, r, size);
        }
        r
    }

    /// Routes a layout to the smallest slab tier that can serve it.
    /// Deterministic: the same size always maps to the same tier.
    #[verus_spec(result =>
        ensures
            match result {
                Ok(tier) => {
                    let t = tier as usize;
                    &&& is_supported_tier(t)
                    &&& spec_layout_size(*layout) >= 1
                    &&& t >= spec_layout_size(*layout)
                    &&& forall|s: usize| is_supported_tier(s) && s >= spec_layout_size(*layout) ==> t <= s
                },
                Err(_) => {
                    spec_layout_size(*layout) == 0
                        || spec_layout_size(*layout) > MAX_SLAB_SIZE
                },
            },
    )]
    fn layout_to_allocator(layout: &Layout) -> Result<SlabSize, AllocError> {
        let r = match layout.size() {
            1..=8 => Ok(SlabSize::Slab8),
            9..=16 => Ok(SlabSize::Slab16),
            17..=32 => Ok(SlabSize::Slab32),
            33..=64 => Ok(SlabSize::Slab64),
            65..=128 => Ok(SlabSize::Slab128),
            129..=256 => Ok(SlabSize::Slab256),
            257..=512 => Ok(SlabSize::Slab512),
            _ => Err(AllocError),
        };
        proof! {
            if let Ok(tier) = r {
                lemma_layout_to_allocator_minimality(tier, spec_layout_size(*layout));
            }
        }
        r
    }
}

//==================================================================================================
// GlobalAlloc
//==================================================================================================

unsafe impl GlobalAlloc for ArenaAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let heap = ptr::addr_of_mut!(HEAP);
        if let Some(heap) = &mut *heap {
            match heap.allocate(layout) {
                Ok(ptr) => ptr,
                Err(_) => {
                    error!("allocation failed (layout={:?})", layout);
                    core::ptr::null_mut()
                },
            }
        } else {
            error!("heap is not initialized");
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let heap = ptr::addr_of_mut!(HEAP);
        if let Some(heap) = &mut *heap {
            if let Err(e) = heap.deallocate(ptr, layout) {
                error!("deallocation failed (layout={:?}): {:?}", layout, e);
            }
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Records an externally-provided backing buffer for the kernel heap.
///
/// All platforms must call this function before [`init()`]. The heap initializer
/// unconditionally requires a previously recorded backing buffer and does not
/// fall back to any kheap-local static storage.
///
/// # Parameters
///
/// - `ptr`: Pointer to the start of the backing buffer. Must be page-aligned.
/// - `size`: Size of the backing buffer in bytes. Must be a multiple of [`MIN_HEAP_SIZE`].
///
#[allow(dead_code)]
pub unsafe fn set_backing_storage(ptr: *mut u8, size: usize) -> Result<(), Error> {
    if ptr.is_null() {
        let reason: &str = "null backing storage pointer";
        error!("set_backing_storage(): {}", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    if !(ptr as usize).is_multiple_of(mem::PAGE_SIZE) {
        let reason: &str = "unaligned backing storage pointer";
        error!("set_backing_storage(): {}", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    if size < MIN_HEAP_SIZE {
        let reason: &str = "backing storage too small";
        error!("set_backing_storage(): {} (size={}, min={})", reason, size, MIN_HEAP_SIZE);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    if !size.is_multiple_of(MIN_HEAP_SIZE) {
        let reason: &str = "backing storage size is not a multiple of MIN_HEAP_SIZE";
        error!("set_backing_storage(): {} (size={}, min={})", reason, size, MIN_HEAP_SIZE);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }
    BACKING_PTR = ptr;
    BACKING_SIZE = size;
    Ok(())
}

pub unsafe fn init() -> Result<(), Error> {
    info!("initializing the kernel heap...");

    if BACKING_PTR.is_null() {
        let reason: &str = "backing storage not set";
        error!("init(): {}", reason);
        return Err(Error::new(ErrorCode::InvalidArgument, reason));
    }

    HEAP = Some(Kheap::from_raw_parts(BACKING_PTR as usize, BACKING_SIZE)?);

    Ok(())
}
