// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Slab - Specifications
//
// This file contains specification functions, SlabView, and View trait for Slab.

verus! {

pub assume_specification<T: core::marker::PointeeSized> [ <*mut T>::is_null ] (ptr: *mut T) -> (result: bool)
    ensures
        result == (ptr as usize == 0),
;

pub assume_specification<T: Sized> [ <*mut T>::wrapping_add ] (p: *mut T, count: usize) -> (result: *mut T)
    ensures
        result as usize == ((p as usize + count * size_of::<T>()) % (usize::MAX + 1)) as usize,
;

pub assume_specification<T: core::marker::PointeeSized>[ <*mut T as core::cmp::PartialOrd>::lt ] (
    p: &*mut T,
    q: &*mut T
) -> (result: bool)
    ensures
        result == ((*p as usize) < (*q as usize)),
;

pub assume_specification<T: core::marker::PointeeSized>[ <*const T as core::cmp::PartialOrd>::lt ] (
    p: &*const T,
    q: &*const T
) -> (result: bool)
    ensures
        result == ((*p as usize) < (*q as usize)),
;

pub assume_specification<T: core::marker::PointeeSized>[ <*const T as core::cmp::PartialOrd>::ge ] (
    p: &*const T,
    q: &*const T
) -> (result: bool)
    ensures
        result == ((*p as usize) >= (*q as usize)),
;

pub assume_specification<T: Sized> [ <*mut T>::add ] (p: *mut T, count: usize) -> (result: *mut T)
    requires
        p as usize + count * size_of::<T>() <= usize::MAX,
        count * size_of::<T>() <= isize::MAX,
    ensures
        result as usize == p as usize + count * size_of::<T>(),
;

pub assume_specification<T: Sized> [ <*const T>::offset_from_unsigned ] (
    p: *const T,
    origin: *const T
) -> (result: usize)
    requires
        p as usize >= origin as usize,
        // The difference must be bounded by isize::MAX, according to
        // https://doc.rust-lang.org/src/core/ptr/const_ptr.rs.html#672-675
        p as usize - origin as usize <= isize::MAX,
        (p as usize - origin as usize) % (size_of::<T>() as int) == 0,
    ensures
        result == (p as usize - origin as usize) / (size_of::<T>() as int),
;

pub axiom fn axiom_align_of_u8_is_1()
    ensures
        vstd::layout::align_of::<u8>() == 1,
;

// A view of the Slab as a set of blocks, each either allocated or freed.
#[verifier::ext_equal]
pub struct SlabView {
    pub block_size: usize,
    pub start_addr: usize,
    pub end_addr: usize,
    pub allocated_addrs: Set<usize>,
    pub free_addrs: Set<usize>,
}

impl SlabView {
    pub open spec fn inv(&self) -> bool {
        &&& self.block_size > 0
        &&& self.start_addr % self.block_size == 0
        &&& self.end_addr % self.block_size == 0
        &&& self.end_addr > self.start_addr
        &&& forall|i| #[trigger] self.allocated_addrs.contains(i) ==> {
            &&& self.start_addr <= i < self.end_addr
            &&& i % self.block_size == 0
        }
        &&& forall|i| #[trigger] self.free_addrs.contains(i) ==> {
            &&& self.start_addr <= i < self.end_addr
            &&& i % self.block_size == 0
        }
        &&& self.allocated_addrs.disjoint(self.free_addrs)
        // Completeness: every block-aligned address in range is either allocated or free.
        &&& forall|a: usize| a % self.block_size == 0 && self.start_addr <= a < self.end_addr
                ==> #[trigger] self.allocated_addrs.contains(a) || self.free_addrs.contains(a)
    }
}

}
