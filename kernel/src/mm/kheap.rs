// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    arch::mem,
    klib::{
        self,
        slab::Slab,
    },
};
use ::alloc::alloc::{
    AllocError,
    GlobalAlloc,
    Layout,
};
use ::core::ptr;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

pub const NUM_OF_SLABS: usize = 8;
const SLAB_COUNT: usize = 32;
pub const MIN_SLAB_SIZE: usize = SLAB_COUNT * mem::PAGE_SIZE;
pub const MIN_HEAP_SIZE: usize = NUM_OF_SLABS * MIN_SLAB_SIZE;

//==================================================================================================
//  Structures
//==================================================================================================

struct ArenaAllocator;

#[repr(align(4096))]
struct HeapStorage {
    memory: [u8; MIN_HEAP_SIZE],
}

static_assert_alignment!(HeapStorage, mem::PAGE_SIZE);

static mut HEAP_STORAGE: HeapStorage = HeapStorage {
    memory: [0; MIN_HEAP_SIZE],
};

#[derive(Copy, Clone)]
enum SlabSize {
    Slab8 = 8,
    Slab16 = 16,
    Slab32 = 32,
    Slab64 = 64,
    Slab128 = 128,
    Slab256 = 256,
    Slab512 = 512,
    Slab4096 = 4096,
}

struct Kheap {
    slab_8_bytes: Slab,
    slab_16_bytes: Slab,
    slab_32_bytes: Slab,
    slab_64_bytes: Slab,
    slab_128_bytes: Slab,
    slab_256_bytes: Slab,
    slab_512_bytes: Slab,
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

impl Kheap {
    unsafe fn from_raw_parts(addr: usize, size: usize) -> Result<Kheap, Error> {
        // Check if start address is not page aligned.
        if addr % 4096 != 0 {
            return Err(Error::new(ErrorCode::InvalidArgument, "unaligned start address"));
        }

        // Check if size is less than minimum heap size.
        if size < MIN_HEAP_SIZE {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "heap size is less than minimum heap size",
            ));
        }

        // Check if size is not a multiple of heap size.
        if size % MIN_HEAP_SIZE != 0 {
            error!("size is not a multiple of page size");
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "size is not a multiple of page size",
            ));
        }

        let heap_start_addr: *mut u8 = addr as *mut u8;
        let slab_size: usize = size / NUM_OF_SLABS;
        info!("heap size: {} MB", size / klib::MEGABYTE);
        info!("slab size: {} KB", slab_size / klib::KILOBYTE);
        Ok(Kheap {
            slab_8_bytes: Slab::from_raw_parts(heap_start_addr, slab_size, 8)?,
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
            slab_4096_bytes: Slab::from_raw_parts(
                heap_start_addr.add(7 * slab_size),
                slab_size,
                SlabSize::Slab4096 as usize,
            )?,
        })
    }

    unsafe fn allocate(&mut self, layout: Layout) -> Result<*mut u8, AllocError> {
        match Kheap::layout_to_allocator(&layout)? {
            SlabSize::Slab8 => self.slab_8_bytes.allocate().map_err(|_| AllocError),
            SlabSize::Slab16 => self.slab_16_bytes.allocate().map_err(|_| AllocError),
            SlabSize::Slab32 => self.slab_32_bytes.allocate().map_err(|_| AllocError),
            SlabSize::Slab64 => self.slab_64_bytes.allocate().map_err(|_| AllocError),
            SlabSize::Slab128 => self.slab_128_bytes.allocate().map_err(|_| AllocError),
            SlabSize::Slab256 => self.slab_256_bytes.allocate().map_err(|_| AllocError),
            SlabSize::Slab512 => self.slab_512_bytes.allocate().map_err(|_| AllocError),
            SlabSize::Slab4096 => self.slab_4096_bytes.allocate().map_err(|_| AllocError),
        }
    }

    unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) -> Result<(), AllocError> {
        match Kheap::layout_to_allocator(&layout)? {
            SlabSize::Slab8 => self.slab_8_bytes.deallocate(ptr).map_err(|_| AllocError),
            SlabSize::Slab16 => self.slab_16_bytes.deallocate(ptr).map_err(|_| AllocError),
            SlabSize::Slab32 => self.slab_32_bytes.deallocate(ptr).map_err(|_| AllocError),
            SlabSize::Slab64 => self.slab_64_bytes.deallocate(ptr).map_err(|_| AllocError),
            SlabSize::Slab128 => self.slab_128_bytes.deallocate(ptr).map_err(|_| AllocError),
            SlabSize::Slab256 => self.slab_256_bytes.deallocate(ptr).map_err(|_| AllocError),
            SlabSize::Slab512 => self.slab_512_bytes.deallocate(ptr).map_err(|_| AllocError),
            SlabSize::Slab4096 => self.slab_4096_bytes.deallocate(ptr).map_err(|_| AllocError),
        }
    }

    pub fn layout_to_allocator(layout: &Layout) -> Result<SlabSize, AllocError> {
        match layout.size() {
            1..=8 => Ok(SlabSize::Slab8),
            9..=16 => Ok(SlabSize::Slab16),
            17..=32 => Ok(SlabSize::Slab32),
            33..=64 => Ok(SlabSize::Slab64),
            65..=128 => Ok(SlabSize::Slab128),
            129..=256 => Ok(SlabSize::Slab256),
            257..=512 => Ok(SlabSize::Slab512),
            4096 => Ok(SlabSize::Slab4096),
            _ => Err(AllocError),
        }
    }
}

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
