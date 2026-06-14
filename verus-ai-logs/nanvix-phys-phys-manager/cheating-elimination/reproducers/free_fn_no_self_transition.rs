use vstd::prelude::*;
verus! {
pub struct V { pub f: Set<int>, pub a: Set<int> }
impl V {
    pub open spec fn alloc_one(self, x: int) -> V {
        V { f: self.f.remove(x), a: self.a.insert(x) }
    }
}
// Model: manager wraps an opaque pool view; frame::alloc is a FREE function (no &mut self).
pub struct Mgr { pub ghost view: V }
impl Mgr { pub open spec fn vw(self) -> V { self.view } }

// free function, no self — exactly frame::alloc's shape
#[verifier::external_body]
pub fn frame_alloc() -> (r: int) ensures true {  unimplemented!() }

// manager method demanding a self@ transition the free call cannot cause
fn alloc_kernel_frame(m: &mut Mgr) -> (r: int)
    ensures final(m).vw() == old(m).vw().alloc_one(r),
{
    let addr = frame_alloc();   // does NOT touch m
    // m.view is unchanged here; ensures demands it changed. Unprovable without a token.
    addr
}
} // verus!
fn main(){}
