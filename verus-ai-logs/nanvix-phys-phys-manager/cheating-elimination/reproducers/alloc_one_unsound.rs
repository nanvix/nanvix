use vstd::prelude::*;

verus! {

// Minimal model of FrameAllocView (the real one has Set<int>/Map<int,int> fields).
pub struct FrameAllocView {
    pub allocated_frames: Set<int>,
    pub free_frames: Set<int>,
    pub refcounts: Map<int, int>,
}

impl FrameAllocView {
    pub open spec fn wf(&self) -> bool {
        self.allocated_frames.disjoint(self.free_frames)
    }
    pub open spec fn alloc_one(self, addr: int) -> FrameAllocView {
        FrameAllocView {
            allocated_frames: self.allocated_frames.insert(addr),
            free_frames: self.free_frames.remove(addr),
            refcounts: self.refcounts.insert(addr, 1int),
        }
    }
}

// EXACT shape of lemma_kernel_alloc_one (manager.proof.rs:27) turned into an
// external_body axiom (the reviewer's "tcb-allowed boundary" fallback).
#[verifier::external_body]
pub proof fn lemma_kernel_alloc_one(pre: FrameAllocView, post: FrameAllocView, addr: int)
    requires
        pre.wf(),
    ensures
        pre.free_frames.contains(addr),
        post == pre.alloc_one(addr),
        post.wf(),
{
}

// A caller can now derive `false` from this "axiom": pick an empty-free wf partition.
proof fn exploit() ensures false {
    let empty = FrameAllocView {
        allocated_frames: Set::empty(),
        free_frames: Set::empty(),
        refcounts: Map::empty(),
    };
    assert(empty.wf());
    lemma_kernel_alloc_one(empty, empty.alloc_one(0), 0);
    // ensures gave us: empty.free_frames.contains(0), but free_frames is empty.
    assert(empty.free_frames.contains(0));
    assert(!empty.free_frames.contains(0));  // Set::empty() contains nothing
    assert(false);
}

} // verus!

fn main() {}
