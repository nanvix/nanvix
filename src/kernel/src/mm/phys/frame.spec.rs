verus! {

use super::UpoolView;
use crate::hal::mem::spec_page_size;

// ---------------------------------------------------------------------------
// External type specifications (arch crate)
// ---------------------------------------------------------------------------

#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFrameNumber(FrameNumber);

// ---------------------------------------------------------------------------
// Assumed specs for arch crate functions
// ---------------------------------------------------------------------------

pub assume_specification[ ::arch::mem::FRAME_SIZE ] -> (result: usize)
    ensures
        result == spec_page_size(),
        result > 0,
;

pub assume_specification[ FrameNumber::from_raw_value ](value: usize) -> (result: Option<FrameNumber>)
;

pub assume_specification[ FrameNumber::into_raw_value ](self_: FrameNumber) -> (result: usize)
;

// ---------------------------------------------------------------------------
// Assumed specs for kernel HAL functions
// ---------------------------------------------------------------------------

pub assume_specification[ FrameAddress::from_frame_number ](frame_number: FrameNumber) -> (result: Result<FrameAddress, Error>)
;

pub assume_specification[ FrameAddress::into_frame_number ](self_: FrameAddress) -> (result: FrameNumber)
;

pub assume_specification[ PhysicalAddress::into_frame_number ](self_: PhysicalAddress) -> (result: FrameNumber)
;

// Generic trait methods (deref, start, size, into_raw_value) are annotated
// directly on their impl methods with #[verus_verify(external_body)] in
// page.rs and region.rs — assume_specification cannot match generic signatures.

// ---------------------------------------------------------------------------
// Spec transition functions on UpoolView
// ---------------------------------------------------------------------------

impl UpoolView {
    /// Allocate a single frame: move from free to allocated.
    pub open spec fn spec_alloc(self, frame: int) -> UpoolView {
        UpoolView {
            allocated_frames: self.allocated_frames.insert(frame),
            free_frames: self.free_frames.remove(frame),
        }
    }

    /// Free a single frame: move from allocated to free.
    pub open spec fn spec_free(self, frame: int) -> UpoolView {
        UpoolView {
            allocated_frames: self.allocated_frames.remove(frame),
            free_frames: self.free_frames.insert(frame),
        }
    }

    /// Reserve (book) a single frame: move from free to allocated.
    pub open spec fn spec_book(self, addr: int) -> UpoolView {
        UpoolView {
            allocated_frames: self.allocated_frames.insert(addr),
            free_frames: self.free_frames.remove(addr),
        }
    }

    /// Reserve a range of frames given as a set.
    pub open spec fn spec_alloc_range(self, frames: Set<int>) -> UpoolView {
        UpoolView {
            allocated_frames: self.allocated_frames.union(frames),
            free_frames: self.free_frames.difference(frames),
        }
    }
}

// ---------------------------------------------------------------------------
// Inner invariant
// ---------------------------------------------------------------------------

impl Inner {
    pub open spec fn inv(&self) -> bool
    {
        &&& self@.wf()
        &&& self.internal_inv()
        // All tracked addresses are non-negative (physical addresses).
        &&& forall|addr: int| self@.allocated_frames.contains(addr) ==> addr >= 0
        &&& forall|addr: int| self@.free_frames.contains(addr) ==> addr >= 0
    }
}

} // verus!
