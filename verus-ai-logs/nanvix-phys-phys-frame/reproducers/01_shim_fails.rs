// Isolated reproducer for the frame.rs shim discharge obligation.
//
// Models the EXACT architecture:
//   * an `uninterp spec fn` constant view  (== mod.spec.rs `phys_view()`)
//   * an `external_body` accessor `instance()` with `ensures (*r)@ == view().frames`
//   * a mutating method `Inner::alloc` with a frozen old()/final() contract
//   * a shim `alloc()` asserting a POST-state fact about `view()`
//
// Goal: can ANY sound contract on `instance()` (the one modifiable boundary)
// let the shim prove its frozen post-state `ensures`?
use vstd::prelude::*;

verus! {

pub struct FrameAllocView {
    pub allocated_frames: Set<int>,
    pub free_frames: Set<int>,
}

impl FrameAllocView {
    pub open spec fn wf(&self) -> bool {
        // disjointness, exactly as mod.spec.rs FrameAllocView::wf
        self.allocated_frames.disjoint(self.free_frames)
    }
}

pub struct PhysMemView {
    pub initialized: bool,
    pub frames: FrameAllocView,
}

impl PhysMemView {
    pub open spec fn inv(self) -> bool {
        self.initialized ==> self.frames.wf()
    }
}

// The do-not-modify, argument-free constant view (== phys_view()).
pub uninterp spec fn view_fn() -> PhysMemView;

pub struct Inner {
    pub dummy: u8,
}

impl View for Inner {
    type V = FrameAllocView;
    uninterp spec fn view(&self) -> FrameAllocView;
}

impl Inner {
    pub open spec fn inv(&self) -> bool {
        self@.wf()
    }

    // FROZEN contract — identical shape to frame.rs Inner::alloc.
    #[verifier::external_body]
    pub fn alloc(&mut self) -> (result: u64)
        requires
            old(self).inv(),
        ensures
            final(self).inv(),
            // Ok-branch post-state: the returned frame was free, now allocated.
            old(self)@.free_frames.contains(result as int),
            final(self)@ == (FrameAllocView {
                allocated_frames: old(self)@.allocated_frames.insert(result as int),
                free_frames: old(self)@.free_frames.remove(result as int),
            }),
    {
        unimplemented!()
    }
}

// The ONE modifiable boundary: instance().
// Current contract pins the returned ref to the PRE state.
#[verifier::external_body]
pub fn instance() -> (result: &'static mut Inner)
    requires
        view_fn().initialized,
    ensures
        (*result).inv(),
        (*result)@ == view_fn().frames,
{
    unimplemented!()
}

// The shim with the FROZEN post-state ensures (== frame.rs alloc()).
pub fn alloc_shim() -> (result: u64)
    requires
        view_fn().initialized,
        view_fn().inv(),
    ensures
        // FROZEN: post-state membership over the constant view_fn().
        view_fn().frames.allocated_frames.contains(result as int),
{
    instance().alloc()
}

} // verus!

fn main() {}
