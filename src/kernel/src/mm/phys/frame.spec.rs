verus! {

use super::UpoolView;
use crate::hal::mem::spec_page_size;

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
