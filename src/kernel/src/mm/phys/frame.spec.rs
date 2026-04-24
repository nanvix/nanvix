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

// FrameNumber conversion assume_specifications are not needed because
// frame.rs uses wrapper functions (frame_addr_to_bitmap_index, etc.)
// that encapsulate the full conversion chain with external_body.

// Generic trait methods (deref, start, size, into_raw_value) are annotated
// directly on their impl methods with #[verus_verify(external_body)] in
// page.rs and region.rs — assume_specification cannot match generic signatures.

// ---------------------------------------------------------------------------
// Assumed spec for init() — function body uses MaybeUninit::write() which
// Verus cannot compile even with external_body. Spec provided here instead.
// ---------------------------------------------------------------------------

pub assume_specification[ init ](bitmap: SparseBitmap) -> (result: Result<(), Error>)
    ensures
        // Singleton pattern: state not expressible without ghost accessor.
        result.is_ok() || result.is_err(),
;

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
    }
}

} // verus!
