// Reproducer 2: the shim's frozen post-state `ensures` is logically CONTRADICTED
// by its own premises under ANY single-state contract on `instance()`.
//
// In the Ok branch, `Inner::alloc` guarantees the returned `frame` was in
// `old(self)@.free_frames`. The only bridge `instance()` can soundly provide is
// `(*result)@ == view_fn().frames` (a fact about the constant view at the call's
// return). So `frame ∈ view_fn().frames.free_frames`. By `wf` disjointness,
// `frame ∉ view_fn().frames.allocated_frames` — the EXACT negation of the frozen
// shim `ensures`. Hence the shim goal is `false` whenever it is reachable, and no
// proof (nor any consistent strengthening of `instance()`) can discharge it.
use vstd::prelude::*;

verus! {

pub struct FrameAllocView {
    pub allocated_frames: Set<int>,
    pub free_frames: Set<int>,
}

impl FrameAllocView {
    pub open spec fn wf(&self) -> bool {
        self.allocated_frames.disjoint(self.free_frames)
    }
}

pub struct PhysMemView { pub initialized: bool, pub frames: FrameAllocView }

impl PhysMemView {
    pub open spec fn inv(self) -> bool { self.initialized ==> self.frames.wf() }
}

pub uninterp spec fn view_fn() -> PhysMemView;

// This lemma encodes EXACTLY what the shim must prove (`goal`) against EXACTLY
// what it can know (`known`), and shows the two are contradictory.
proof fn shim_goal_is_false(frame: int)
    requires
        // From precondition: the constant view is well-formed.
        view_fn().inv(),
        view_fn().initialized,
        // From `instance()` (the strongest sound single-state bridge) +
        // `Inner::alloc`'s Ok ensures `old(self)@.free_frames.contains(frame)`:
        view_fn().frames.free_frames.contains(frame),
    ensures
        // The frozen shim postcondition is FALSE here:
        !view_fn().frames.allocated_frames.contains(frame),
{
    // wf disjointness directly yields the contradiction with the shim goal.
    assert(view_fn().frames.wf());
    assert(view_fn().frames.allocated_frames.disjoint(view_fn().frames.free_frames));
}

} // verus!

fn main() {}
