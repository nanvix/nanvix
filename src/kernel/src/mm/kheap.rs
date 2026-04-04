// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[allow(unused_imports)]
use ::vstd::prelude::*;

// Include specifications.
#[cfg(verus_keep_ghost)]
include!("kheap.spec.rs");
// Include proofs.
#[cfg(verus_keep_ghost)]
include!("kheap.proof.rs");

use crate::collections::Slab;
#[cfg(verus_keep_ghost)]
use crate::collections::SlabView;
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

//==================================================================================================
// Constants
//==================================================================================================

#[cfg(not(verus_keep_ghost))]
#[cfg(feature = "hyperlight")]
pub const NUM_OF_SLABS: usize = 10;
#[cfg(not(verus_keep_ghost))]
#[cfg(not(feature = "hyperlight"))]
pub const NUM_OF_SLABS: usize = 7;
#[cfg(not(verus_keep_ghost))]
const SLAB_COUNT: usize = 32;
#[cfg(not(verus_keep_ghost))]
pub const MIN_SLAB_SIZE: usize = SLAB_COUNT * mem::PAGE_SIZE;
#[cfg(not(verus_keep_ghost))]
pub const MIN_HEAP_SIZE: usize = NUM_OF_SLABS * MIN_SLAB_SIZE;

//==================================================================================================
//  Structures
//==================================================================================================

struct ArenaAllocator;

#[repr(align(4096))]
struct HeapStorage {
    memory: [u8; MIN_HEAP_SIZE],
}

::static_assert::assert_eq_align!(HeapStorage, mem::PAGE_SIZE);

static mut HEAP_STORAGE: HeapStorage = HeapStorage {
    memory: [0; MIN_HEAP_SIZE],
};

#[cfg(not(verus_keep_ghost))]
#[derive(Copy, Clone)]
enum SlabSize {
    Slab8 = 8,
    Slab16 = 16,
    Slab32 = 32,
    Slab64 = 64,
    Slab128 = 128,
    Slab256 = 256,
    Slab512 = 512,
    #[cfg(feature = "hyperlight")]
    Slab1024 = 1024,
    #[cfg(feature = "hyperlight")]
    Slab2048 = 2048,
    // FIXME (#1780): investigate what causes allocations >512 bytes under hyperlight
    // and remove these extended slab tiers once the root cause is addressed.
    #[cfg(feature = "hyperlight")]
    Slab4096 = 4096,
}

#[cfg(not(verus_keep_ghost))]
struct Kheap {
    slab_8_bytes: Slab,
    slab_16_bytes: Slab,
    slab_32_bytes: Slab,
    slab_64_bytes: Slab,
    slab_128_bytes: Slab,
    slab_256_bytes: Slab,
    slab_512_bytes: Slab,
    #[cfg(feature = "hyperlight")]
    slab_1024_bytes: Slab,
    #[cfg(feature = "hyperlight")]
    slab_2048_bytes: Slab,
    #[cfg(feature = "hyperlight")]
    slab_4096_bytes: Slab,
}

//==================================================================================================
// Global Variables
//==================================================================================================

static mut HEAP: Option<Kheap> = None;

#[global_allocator]
static mut ALLOCATOR: ArenaAllocator = ArenaAllocator;

//==================================================================================================
// Implementations
//==================================================================================================

verus! {

impl Kheap {
    // FN-2: Construct a Kheap by partitioning a raw memory region into slabs.
    unsafe fn from_raw_parts(addr: usize, size: usize) -> (result: Result<Kheap, Error>)
        requires
            // SAF-1: region must not wrap around address space
            addr as int + size as int <= usize::MAX as int,
            // SAF-2: total size must fit in isize for pointer arithmetic
            size as int <= isize::MAX as int,
        ensures
            match result {
                Ok(heap) => {
                    let slab_size = size as int / NUM_OF_SLABS as int;
                    // FN-2b: heap invariant holds
                    &&& heap.inv()
                    // FN-2c: all slabs start fully unallocated
                    &&& forall|i: int| 0 <= i < NUM_OF_SLABS as int ==>
                        (#[trigger] heap@.slabs[i]).allocated_addrs == Set::<usize>::empty()
                    // FN-2e: each slab is contained within its partition
                    &&& forall|i: int| 0 <= i < NUM_OF_SLABS as int ==> {
                        &&& (#[trigger] heap@.slabs[i]).start_addr >= addr as int + i * slab_size
                        &&& heap@.slabs[i].end_addr <= addr as int + (i + 1) * slab_size
                    }
                    // FN-2g (forward): success implies preconditions held
                    &&& addr as int % PAGE_SIZE as int == 0
                    &&& size >= MIN_HEAP_SIZE
                    &&& size as int % MIN_HEAP_SIZE as int == 0
                }
                Err(e) => {
                    // FN-2f: error code
                    &&& e.code == ErrorCode::InvalidArgument
                }
            },
    {
        // Check if start address is not page aligned.
        // VERUS DEVIATION: mem::PAGE_SIZE cfg-gated — defined outside verus! {} block
        if !addr.is_multiple_of({
            #[cfg(not(verus_keep_ghost))]
            { mem::PAGE_SIZE }
            #[cfg(verus_keep_ghost)]
            { PAGE_SIZE }
        }) {
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

        // VERUS DEVIATION: addr as *mut u8 unsupported — Verus lacks usize-to-pointer cast
        let heap_start_addr: *mut u8 = {
            #[cfg(not(verus_keep_ghost))]
            { addr as *mut u8 }
            #[cfg(verus_keep_ghost)]
            { usize_to_mut_ptr(addr) }
        };
        let slab_size: usize = size / NUM_OF_SLABS;
        #[cfg(not(verus_keep_ghost))]
        info!("heap size: {} MB", size / constants::MEGABYTE);
        #[cfg(not(verus_keep_ghost))]
        info!("slab size: {} KB", slab_size / constants::KILOBYTE);
        proof {
            broadcast use vstd::std_specs::control_flow::group_control_flow_axioms;
            assert(size_of::<u8>() == 1) by {
                broadcast use vstd::layout::layout_of_primitives;
            };
            vstd::arithmetic::div_mod::lemma_fundamental_div_mod(size as int, NUM_OF_SLABS as int);
            vstd::arithmetic::div_mod::lemma_mod_pos_bound(size as int, NUM_OF_SLABS as int);
            assert(slab_size as int * NUM_OF_SLABS as int <= size as int);
            assert(heap_start_addr as usize == addr);
            assert(1 * slab_size as int <= size as int);
            assert(2 * slab_size as int <= size as int);
            assert(3 * slab_size as int <= size as int);
            assert(4 * slab_size as int <= size as int);
            assert(5 * slab_size as int <= size as int);
            assert(6 * slab_size as int <= size as int);
        }
        Ok(Kheap {
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
            #[cfg(feature = "hyperlight")]
            slab_1024_bytes: Slab::from_raw_parts(
                heap_start_addr.add(7 * slab_size),
                slab_size,
                SlabSize::Slab1024 as usize,
            )?,
            #[cfg(feature = "hyperlight")]
            slab_2048_bytes: Slab::from_raw_parts(
                heap_start_addr.add(8 * slab_size),
                slab_size,
                SlabSize::Slab2048 as usize,
            )?,
            #[cfg(feature = "hyperlight")]
            slab_4096_bytes: Slab::from_raw_parts(
                heap_start_addr.add(9 * slab_size),
                slab_size,
                SlabSize::Slab4096 as usize,
            )?,
        })
    }

    // FN-3: Allocate a block from the slab matching layout.size().
    unsafe fn allocate(&mut self, layout: Layout) -> (result: Result<*mut u8, AllocError>)
        requires
            // FN-3a
            old(self).inv(),
        ensures
            // FN-3e: invariant preserved
            self.inv(),
            match result {
                Ok(ptr) => {
                    let opt_idx = spec_slab_for_size(spec_layout_size(layout) as int);
                    // FN-3b: address was free in the correct slab
                    &&& opt_idx.is_some()
                    &&& old(self)@.slabs[opt_idx.unwrap()].free_addrs.contains(ptr as usize)
                    // FN-3c: pointer is block-aligned
                    &&& ptr as usize % old(self)@.slabs[opt_idx.unwrap()].block_size == 0
                    // FN-3d: exact state transition
                    &&& self@ == old(self)@.spec_allocate(opt_idx.unwrap(), ptr as usize)
                }
                Err(_) => {
                    let opt_idx = spec_slab_for_size(spec_layout_size(layout) as int);
                    // FN-3g: state preserved on error
                    &&& self@ == old(self)@
                    // FN-3f: error iff size unsupported or slab exhausted
                    &&& (opt_idx.is_none()
                        || old(self)@.slabs[opt_idx.unwrap()].free_addrs
                            == Set::<usize>::empty())
                }
            },
    {
        proof {
            broadcast use vstd::std_specs::control_flow::group_control_flow_axioms;
        }
        // VERUS DEVIATION: |_| → |_e| — Verus requires named variables in closure params
        match Kheap::layout_to_allocator(&layout)? {
            SlabSize::Slab8 => self.slab_8_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab16 => self.slab_16_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab32 => self.slab_32_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab64 => self.slab_64_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab128 => self.slab_128_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab256 => self.slab_256_bytes.allocate().map_err(|_e| AllocError),
            SlabSize::Slab512 => self.slab_512_bytes.allocate().map_err(|_e| AllocError),
            #[cfg(feature = "hyperlight")]
            SlabSize::Slab1024 => self.slab_1024_bytes.allocate().map_err(|_e| AllocError),
            #[cfg(feature = "hyperlight")]
            SlabSize::Slab2048 => self.slab_2048_bytes.allocate().map_err(|_e| AllocError),
            #[cfg(feature = "hyperlight")]
            SlabSize::Slab4096 => self.slab_4096_bytes.allocate().map_err(|_e| AllocError),
        }
    }

    // FN-4: Return a previously-allocated block to its slab.
    unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) -> (result: Result<(), AllocError>)
        requires
            // FN-4a
            old(self).inv(),
        ensures
            // FN-4d: invariant preserved
            self.inv(),
            match result {
                Ok(()) => {
                    let opt_idx = spec_slab_for_size(spec_layout_size(layout) as int);
                    // FN-4b: pointer was allocated in the correct slab
                    &&& opt_idx.is_some()
                    &&& old(self)@.slabs[opt_idx.unwrap()].allocated_addrs.contains(ptr as usize)
                    // FN-4c: exact state transition
                    &&& self@ == old(self)@.spec_deallocate(opt_idx.unwrap(), ptr as usize)
                }
                Err(_) => {
                    let opt_idx = spec_slab_for_size(spec_layout_size(layout) as int);
                    // FN-4f: state preserved on error
                    &&& self@ == old(self)@
                    // FN-4e: error iff size unsupported or ptr not allocated
                    &&& (opt_idx.is_none()
                        || !old(self)@.slabs[opt_idx.unwrap()].allocated_addrs
                            .contains(ptr as usize))
                }
            },
    {
        proof {
            broadcast use vstd::std_specs::control_flow::group_control_flow_axioms;
        }
        // VERUS DEVIATION: |_| → |_e| — Verus requires named variables in closure params
        match Kheap::layout_to_allocator(&layout)? {
            SlabSize::Slab8 => self.slab_8_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab16 => self.slab_16_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab32 => self.slab_32_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab64 => self.slab_64_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab128 => self.slab_128_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab256 => self.slab_256_bytes.deallocate(ptr).map_err(|_e| AllocError),
            SlabSize::Slab512 => self.slab_512_bytes.deallocate(ptr).map_err(|_e| AllocError),
            #[cfg(feature = "hyperlight")]
            SlabSize::Slab1024 => self.slab_1024_bytes.deallocate(ptr).map_err(|_e| AllocError),
            #[cfg(feature = "hyperlight")]
            SlabSize::Slab2048 => self.slab_2048_bytes.deallocate(ptr).map_err(|_e| AllocError),
            #[cfg(feature = "hyperlight")]
            SlabSize::Slab4096 => self.slab_4096_bytes.deallocate(ptr).map_err(|_e| AllocError),
        }
    }

    // FN-1: Pure routing function. Maps layout size to slab tier.
    pub fn layout_to_allocator(layout: &Layout) -> (result: Result<SlabSize, AllocError>)
        ensures
            match result {
                Ok(ss) => {
                    let opt_idx = spec_slab_for_size(spec_layout_size(*layout) as int);
                    // FN-1a: size is supported
                    &&& opt_idx.is_some()
                    // FN-1b: the matching slab tier is large enough
                    &&& block_sizes()[opt_idx.unwrap()] >= spec_layout_size(*layout) as int
                    // FN-1c: returned SlabSize corresponds to the correct index
                    &&& opt_idx.unwrap() == spec_slab_size_to_index(ss)
                    // FN-1c strengthened: tightest fit — all smaller tiers are too small
                    &&& forall|j: int| 0 <= j < opt_idx.unwrap() ==>
                        block_sizes()[j] < spec_layout_size(*layout) as int
                }
                // FN-1d: error iff size is unsupported
                Err(_) => spec_slab_for_size(spec_layout_size(*layout) as int).is_none(),
            },
    {
        match layout.size() {
            1..=8 => Ok(SlabSize::Slab8),
            9..=16 => Ok(SlabSize::Slab16),
            17..=32 => Ok(SlabSize::Slab32),
            33..=64 => Ok(SlabSize::Slab64),
            65..=128 => Ok(SlabSize::Slab128),
            129..=256 => Ok(SlabSize::Slab256),
            257..=512 => Ok(SlabSize::Slab512),
            #[cfg(feature = "hyperlight")]
            513..=1024 => Ok(SlabSize::Slab1024),
            #[cfg(feature = "hyperlight")]
            1025..=2048 => Ok(SlabSize::Slab2048),
            #[cfg(feature = "hyperlight")]
            2049..=4096 => Ok(SlabSize::Slab4096),
            _ => Err(AllocError),
        }
    }
}

} // verus!

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

pub unsafe fn init() -> Result<(), Error> {
    info!("initializing the kernel heap...");

    HEAP = Some(Kheap::from_raw_parts(
        HEAP_STORAGE.memory.as_ptr() as usize,
        HEAP_STORAGE.memory.len(),
    )?);

    Ok(())
}
