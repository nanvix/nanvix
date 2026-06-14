// Reproducer 3: the only `instance()` strengthening able to discharge the shim is
// a contract that reflects the POST-mutation state back into the constant
// `view_fn()`. Because `view_fn()` is a single argument-free constant, evaluating
// that bridge at two program points (pre- and post-mutation) forces
// pre_state == post_state, i.e. `false`, for any state-changing operation.
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

// The "post-state reflecting" bridge the shim WOULD need: a predicate asserting
// that `view_fn().frames` equals the abstract state `s` observed at a program
// point. (This is the only thing that could let the shim conclude a post-state
// membership fact about the constant `view_fn()`.)
pub open spec fn bridge(s: FrameAllocView) -> bool {
    view_fn().frames == s
}

// Two bridge evaluations across a state-changing step:
//   pre  : bridge(pre_state)
//   post : bridge(post_state)   where post_state != pre_state (a frame moved)
// Together they derive `false`.
proof fn two_bridge_evaluations_derive_false(
    pre_state: FrameAllocView,
    post_state: FrameAllocView,
    frame: int,
)
    requires
        // pre/post differ exactly as `Inner::alloc` mandates (frame moved
        // free -> allocated), so they are genuinely distinct states:
        pre_state.wf(),
        pre_state.free_frames.contains(frame),
        post_state == (FrameAllocView {
            allocated_frames: pre_state.allocated_frames.insert(frame),
            free_frames: pre_state.free_frames.remove(frame),
        }),
        // The shim observes the bridge BEFORE the mutation (via `instance()`)...
        bridge(pre_state),
        // ...and would need it AFTER the mutation to discharge its post-state
        // ensures. Both reference the SAME constant `view_fn()`:
        bridge(post_state),
    ensures
        false,
{
    // bridge(pre) && bridge(post) => pre_state == post_state.
    assert(pre_state == post_state);
    // But `frame` is free in pre and not free in post: contradiction.
    assert(pre_state.free_frames.contains(frame));
    assert(!post_state.free_frames.contains(frame));
}

} // verus!

fn main() {}
