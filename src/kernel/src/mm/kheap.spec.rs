verus! {

//==================================================================================================
// Constants (mirrors of exec constants for Verus visibility)
//==================================================================================================

#[cfg(feature = "hyperlight")]
pub const NUM_OF_SLABS: usize = 10;
#[cfg(not(feature = "hyperlight"))]
pub const NUM_OF_SLABS: usize = 7;
pub const SLAB_COUNT: usize = 32;
// PAGE_SIZE = 4096 (from arch::mem)
pub const PAGE_SIZE: usize = 4096;
pub const MIN_SLAB_SIZE: usize = SLAB_COUNT * PAGE_SIZE;
pub const MIN_HEAP_SIZE: usize = NUM_OF_SLABS * MIN_SLAB_SIZE;

//==================================================================================================
// Type Definitions (mirrors for Verus visibility)
//==================================================================================================

#[derive(Copy, Clone)]
pub(crate) enum SlabSize {
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
    #[cfg(feature = "hyperlight")]
    Slab4096 = 4096,
}

pub(crate) struct Kheap {
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
// External Type Specifications
//==================================================================================================

// Layout from core::alloc — opaque, no field access needed.
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExLayout(Layout);

// AllocError from core::alloc — unit struct, transparent.
#[verifier::external_type_specification]
pub struct ExAllocError(AllocError);

// Error from sys::error — transparent (public fields).
#[verifier::external_type_specification]
pub struct ExError(Error);

// ErrorCode from sys::error — match upstream: transparent.
#[verifier::external_type_specification]
pub struct ExErrorCode(ErrorCode);

//==================================================================================================
// Assume Specifications (Human-Approved)
//==================================================================================================

/// Uninterpreted spec function for Layout::size().
pub uninterp spec fn spec_layout_size(layout: Layout) -> usize;

/// Layout::size() accessor.
pub assume_specification[ Layout::size ](layout: &Layout) -> (result: usize)
    ensures result == spec_layout_size(*layout),
;

/// Error::new constructor.
pub assume_specification[ Error::new ](code: ErrorCode, reason: &'static str) -> (result: Error)
    ensures
        result.code == code,
;

/// core::ptr::null_mut — result is the null pointer.
/// Helper to convert usize to *mut u8 (Verus doesn't support `as *mut u8` cast).
#[verifier::external_body]
fn usize_to_mut_ptr(addr: usize) -> (result: *mut u8)
    ensures result as usize == addr,
{
    addr as *mut u8
}

/// <*mut u8>::with_addr — not needed, using usize_to_mut_ptr instead.
/// usize::is_multiple_of — already in vstd, visible cross-crate.

//==================================================================================================
// Spec Constants
//==================================================================================================

/// Maximum allocation size supported by the heap.
#[cfg(not(feature = "hyperlight"))]
pub open spec fn max_slab_size() -> int { 512 }

#[cfg(feature = "hyperlight")]
pub open spec fn max_slab_size() -> int { 4096 }

/// Maps a SlabSize enum value to its slab index.
#[cfg(not(feature = "hyperlight"))]
pub closed spec fn spec_slab_size_to_index(ss: SlabSize) -> int {
    match ss {
        SlabSize::Slab8 => 0,
        SlabSize::Slab16 => 1,
        SlabSize::Slab32 => 2,
        SlabSize::Slab64 => 3,
        SlabSize::Slab128 => 4,
        SlabSize::Slab256 => 5,
        SlabSize::Slab512 => 6,
    }
}

#[cfg(feature = "hyperlight")]
pub closed spec fn spec_slab_size_to_index(ss: SlabSize) -> int {
    match ss {
        SlabSize::Slab8 => 0,
        SlabSize::Slab16 => 1,
        SlabSize::Slab32 => 2,
        SlabSize::Slab64 => 3,
        SlabSize::Slab128 => 4,
        SlabSize::Slab256 => 5,
        SlabSize::Slab512 => 6,
        SlabSize::Slab1024 => 7,
        SlabSize::Slab2048 => 8,
        SlabSize::Slab4096 => 9,
    }
}

/// The expected block size for each slab index.
#[cfg(not(feature = "hyperlight"))]
pub open spec fn block_sizes() -> Seq<int> {
    seq![8int, 16, 32, 64, 128, 256, 512]
}

#[cfg(feature = "hyperlight")]
pub open spec fn block_sizes() -> Seq<int> {
    seq![8int, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096]
}

/// Maps allocation size to slab index. Mirrors layout_to_allocator logic.
/// Returns None for unsupported sizes (0 or > max).
#[cfg(not(feature = "hyperlight"))]
pub open spec fn spec_slab_for_size(size: int) -> Option<int> {
    if 1 <= size <= 8 { Some(0int) }
    else if 9 <= size <= 16 { Some(1int) }
    else if 17 <= size <= 32 { Some(2int) }
    else if 33 <= size <= 64 { Some(3int) }
    else if 65 <= size <= 128 { Some(4int) }
    else if 129 <= size <= 256 { Some(5int) }
    else if 257 <= size <= 512 { Some(6int) }
    else { None }
}

#[cfg(feature = "hyperlight")]
pub open spec fn spec_slab_for_size(size: int) -> Option<int> {
    if 1 <= size <= 8 { Some(0int) }
    else if 9 <= size <= 16 { Some(1int) }
    else if 17 <= size <= 32 { Some(2int) }
    else if 33 <= size <= 64 { Some(3int) }
    else if 65 <= size <= 128 { Some(4int) }
    else if 129 <= size <= 256 { Some(5int) }
    else if 257 <= size <= 512 { Some(6int) }
    else if 513 <= size <= 1024 { Some(7int) }
    else if 1025 <= size <= 2048 { Some(8int) }
    else if 2049 <= size <= 4096 { Some(9int) }
    else { None }
}

//==================================================================================================
// KheapView — Abstract State
//==================================================================================================

/// Abstract view of the kernel heap as a sequence of slab views.
#[verifier::ext_equal]
pub struct KheapView {
    pub slabs: Seq<SlabView>,
}

impl KheapView {
    /// Structural invariant covering TYPE-1, TYPE-2, TYPE-3.
    pub open spec fn inv(&self) -> bool {
        // TYPE-1: Correct number of slabs
        &&& self.slabs.len() == NUM_OF_SLABS as int
        // TYPE-1: Each slab satisfies its own invariant
        &&& forall|i: int| 0 <= i < self.slabs.len() ==>
            (#[trigger] self.slabs[i]).inv()
        // TYPE-2: Slab regions are contiguous and non-overlapping
        &&& forall|i: int| 0 <= i < self.slabs.len() - 1 ==>
            (#[trigger] self.slabs[i]).end_addr <= self.slabs[i + 1].start_addr
        // TYPE-3: Block sizes match the expected power-of-two sequence
        &&& forall|i: int| 0 <= i < self.slabs.len() ==>
            (#[trigger] self.slabs[i]).block_size == block_sizes()[i]
    }

    /// Union of all allocated addresses across all slabs.
    pub open spec fn all_allocated(&self) -> Set<usize> {
        Set::new(|addr: usize| exists|i: int|
            0 <= i < self.slabs.len()
            && (#[trigger] self.slabs[i]).allocated_addrs.contains(addr))
    }

    /// Union of all free addresses across all slabs.
    pub open spec fn all_free(&self) -> Set<usize> {
        Set::new(|addr: usize| exists|i: int|
            0 <= i < self.slabs.len()
            && (#[trigger] self.slabs[i]).free_addrs.contains(addr))
    }

    /// State transition: allocate addr from slab at idx (FN-3d).
    pub open spec fn spec_allocate(self, idx: int, addr: usize) -> KheapView {
        KheapView {
            slabs: self.slabs.update(idx, SlabView {
                allocated_addrs: self.slabs[idx].allocated_addrs.insert(addr),
                free_addrs: self.slabs[idx].free_addrs.remove(addr),
                ..self.slabs[idx]
            }),
        }
    }

    /// State transition: deallocate addr back to slab at idx (FN-4c).
    pub open spec fn spec_deallocate(self, idx: int, addr: usize) -> KheapView {
        KheapView {
            slabs: self.slabs.update(idx, SlabView {
                allocated_addrs: self.slabs[idx].allocated_addrs.remove(addr),
                free_addrs: self.slabs[idx].free_addrs.insert(addr),
                ..self.slabs[idx]
            }),
        }
    }
}

//==================================================================================================
// View Implementation for Kheap
//==================================================================================================

impl View for Kheap {
    type V = KheapView;

    #[cfg(not(feature = "hyperlight"))]
    closed spec fn view(&self) -> KheapView {
        KheapView {
            slabs: seq![
                self.slab_8_bytes@,
                self.slab_16_bytes@,
                self.slab_32_bytes@,
                self.slab_64_bytes@,
                self.slab_128_bytes@,
                self.slab_256_bytes@,
                self.slab_512_bytes@,
            ],
        }
    }

    #[cfg(feature = "hyperlight")]
    closed spec fn view(&self) -> KheapView {
        KheapView {
            slabs: seq![
                self.slab_8_bytes@,
                self.slab_16_bytes@,
                self.slab_32_bytes@,
                self.slab_64_bytes@,
                self.slab_128_bytes@,
                self.slab_256_bytes@,
                self.slab_512_bytes@,
                self.slab_1024_bytes@,
                self.slab_2048_bytes@,
                self.slab_4096_bytes@,
            ],
        }
    }
}

//==================================================================================================
// Kheap Invariant
//==================================================================================================

impl Kheap {
    pub open spec fn inv(&self) -> bool {
        &&& self@.inv()
        &&& self.concrete_inv()
    }

    #[cfg(not(feature = "hyperlight"))]
    pub closed spec fn concrete_inv(&self) -> bool {
        &&& self.slab_8_bytes.inv()
        &&& self.slab_16_bytes.inv()
        &&& self.slab_32_bytes.inv()
        &&& self.slab_64_bytes.inv()
        &&& self.slab_128_bytes.inv()
        &&& self.slab_256_bytes.inv()
        &&& self.slab_512_bytes.inv()
    }

    #[cfg(feature = "hyperlight")]
    pub closed spec fn concrete_inv(&self) -> bool {
        &&& self.slab_8_bytes.inv()
        &&& self.slab_16_bytes.inv()
        &&& self.slab_32_bytes.inv()
        &&& self.slab_64_bytes.inv()
        &&& self.slab_128_bytes.inv()
        &&& self.slab_256_bytes.inv()
        &&& self.slab_512_bytes.inv()
        &&& self.slab_1024_bytes.inv()
        &&& self.slab_2048_bytes.inv()
        &&& self.slab_4096_bytes.inv()
    }
}

} // verus!
