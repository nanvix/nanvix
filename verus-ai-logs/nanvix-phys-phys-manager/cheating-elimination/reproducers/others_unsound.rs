use vstd::prelude::*;
verus! {
pub struct FrameAllocView {
    pub allocated_frames: Set<int>,
    pub free_frames: Set<int>,
    pub refcounts: Map<int, int>,
}
impl FrameAllocView {
    pub open spec fn wf(&self) -> bool { self.allocated_frames.disjoint(self.free_frames) }
}

// Shape of lemma_user_bulk_err_restored (manager.proof.rs:210) as external_body axiom.
// ensures m@ == pre, requires pre.wf(). Model m@ by an uninterp constant.
pub uninterp spec fn m_view() -> FrameAllocView;

#[verifier::external_body]
pub proof fn lemma_user_bulk_err_restored(pre: FrameAllocView)
    requires pre.wf(),
    ensures m_view() == pre,
{}

proof fn exploit_err_restored() ensures false {
    let p1 = FrameAllocView { allocated_frames: Set::empty(), free_frames: Set::empty(), refcounts: Map::empty() };
    let p2 = FrameAllocView { allocated_frames: Set::empty(), free_frames: Set::empty().insert(0), refcounts: Map::empty() };
    assert(p1.wf());
    assert(p2.wf());
    lemma_user_bulk_err_restored(p1);   // m_view() == p1
    lemma_user_bulk_err_restored(p2);   // m_view() == p2
    assert(p1 == p2);                    // hence p1 == p2
    assert(p1.free_frames =~= p2.free_frames);
    assert(!p1.free_frames.contains(0));
    assert(p2.free_frames.contains(0));
    assert(false);
}
} // verus!
fn main() {}
