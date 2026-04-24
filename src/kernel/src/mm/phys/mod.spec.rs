// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysMemoryManager - Specifications
//
// This file contains specification functions and view types.

verus! {

use crate::hal::mem::spec_page_size;

pub uninterp spec fn byte_at_address(ptr: int) -> u8;

#[verifier::external_type_specification]
pub struct ExErrorCode(sys::error::ErrorCode);
#[verifier::external_type_specification]
pub struct ExError(sys::error::Error);

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct ExRefCell<T: core::marker::MetaSized>(core::cell::RefCell<T>);

pub struct KpoolView {
    pub start: int,
    pub num_pages: int,
    pub used_page_indices: Set<int>,
}

impl KpoolView {

    pub open spec fn wf(&self) -> bool
    {
        &&& self.num_pages > 0
        &&& self.start % spec_page_size() == 0
        &&& forall|i: int| self.used_page_indices.contains(i) ==> 0 <= i < self.num_pages
    }

    pub open spec fn range_free(&self, first_page_index: int, count: int) -> bool
    {
        &&& count > 0
        &&& 0 <= first_page_index <= self.num_pages - count
        &&& forall|i: int| first_page_index <= i < first_page_index + count ==> !self.used_page_indices.contains(i)
    }
    
}

pub struct UpoolView
{
    pub allocated_frames: Set<int>,
    pub free_frames: Set<int>,
}

impl UpoolView
{
    pub open spec fn wf(&self) -> bool
    {
        &&& forall|addr: int| self.allocated_frames.contains(addr) ==> addr % spec_page_size() == 0
        &&& forall|addr: int| self.free_frames.contains(addr) ==> addr % spec_page_size() == 0
        &&& self.allocated_frames.disjoint(self.free_frames)
    }
}

} // end verus!

