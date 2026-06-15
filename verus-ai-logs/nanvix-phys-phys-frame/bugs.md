# Bugs / specification limitations — `src/kernel/src/mm/phys/frame.rs`

No *code* bugs were found (no overflow, off-by-one, missing bounds check, or
unchecked cast). The earlier **specification-architecture limitation** (post-state
effects inexpressible over a constant `phys_view()`) is now **RESOLVED** for the
verified mutating path by the `tracked PhysAuth` carrier; the residual `Drop`-only
exception is documented below.

## RESOLVED: post-state effects via the `tracked PhysAuth` carrier

The mutating reservation shims used to relate their postconditions to the
abstraction only through the argument-free constant

    pub uninterp spec fn phys_view() -> PhysMemView;   // mod.spec.rs

which has the same value at every program point and therefore cannot diff the pre-
and post-state of a mutation. The fix introduces a tracked authority carrier
(`view_design.md`):

    pub tracked struct PhysAuth { pub ghost v: PhysMemView }   // mod.spec.rs
    impl View for PhysAuth { type V = PhysMemView; ... }
    impl PhysAuth { pub open spec fn inv(self) -> bool { self.v.inv() } }

plus the transition methods `spec_alloc_one` / `spec_alloc_set` / `spec_share` /
`spec_free` on `PhysMemView`. The carrier is threaded through `&mut` into the
mutating shims, so each names **two** program points — `old(auth)@` (pre) and
`final(auth)@` (post) — and asserts the exact `FrameAllocView` transition.

The shim body keeps `instance()` unchanged and re-synchronizes the ghost carrier to
the live post-state:

    let r = instance();                  // (*r)@ == phys_view().frames == old(auth)@.frames
    let res = r.op(..);                  // final(r)@ == <Inner transition>
    proof! { auth.v.frames = (*r)@; }    // carrier := post-state
    res

### Strengthened mutating reservation shims (now STRONG, no `admit`)

| shim              | Ok-arm (post-state, TRUE & provable)                                              | Err-arm (TRUE)                          |
|-------------------|-----------------------------------------------------------------------------------|-----------------------------------------|
| alloc             | frame.inv() and final(auth)@ == old(auth)@.spec_alloc_one(frame@) and allocated.contains(frame@) and refcounts[frame@]==1 | final(auth)@ == old(auth)@              |
| alloc_contiguous  | base.inv() and final(auth)@ == old(auth)@.spec_alloc_set({base+i*page}) and subset.allocated, each refcount 1            | final(auth)@ == old(auth)@              |
| book              | final(auth)@ == old(auth)@.spec_alloc_one(phys_addr@) and allocated.contains and refcounts==1                            | final(auth)@==old(auth)@ and !free.contains |
| alloc_range       | final(auth)@ == old(auth)@.spec_alloc_set(region_frames) and region_frames.subset_of(allocated)                          | final(auth)@==old(auth)@ and !subset.free   |
| share             | final(auth)@ == old(auth)@.spec_share(frame@) and allocated.contains and refcounts.contains_key                          | final(auth)@==old(auth)@ and (!allocated or refcount>=255) |

### Cascade into verified callers (now carry the STRONG post-state)

- `Upool::alloc` threads `Tracked(&mut PhysAuth)` and ensures
  `Ok(uf) => final(auth)@ == old(auth)@.spec_alloc_one(uf@) && allocated.contains(uf@) && refcounts[uf@]==1`.
- `UserFrame::share` threads the carrier and ensures
  `Ok(handle) => handle@==self@ && final(auth)@ == old(auth)@.spec_share(self@) && allocated.contains(handle@)`.

Unverified callers (`mm/virt`, `test.rs`) and the `external_body` callers
(`manager::alloc_*`, `mod::book_*`) call the tokenless version of each shim
(generated automatically by the `with` clause) and are unaffected.

## Remaining `Drop`-only exception: `free` (now `external_body`, `admit=0`)

`free` is reached only from `UserFrame::drop` / `KernelFrame::drop`, whose
trait-fixed `drop(&mut self)` signature is `opens_invariants none` + `no_unwind`
and therefore **cannot** receive a `Tracked<&mut PhysAuth>` carrier nor open a
global invariant. Without the token its body cannot discharge `instance()`'s
`phys_view().initialized` precondition nor `Inner::free`'s `frame.inv()`
precondition, and the contract must stay precondition-free for `Drop` soundness.

`free` is therefore `external_body` (see `tcb-allowed.md`) with its weak,
always-true `ensures phys_view().inv()` (`opens_invariants none`, `no_unwind`)
honored as a trust boundary. This **replaces the previous `proof! { admit(); }`**:
the trust surface is identical, but `external_body` is the governed mechanism and
`admit` is now **0**. The precise per-reference `free` transition would need a
per-handle `Tracked<FrameRef>` permission consumed in `Drop` (the Rc-style
pattern); it is not required by any documented caller and is left out to keep the
change minimal.

## Trusted boundary retained (per `tcb-allowed.md`)

The `manager::alloc_user_frame` / `alloc_many_user_frames` / `alloc_kernel_frame`
/ `alloc_many_kernel_frames` methods and the `mod::book_*` reservers stay
`external_body` (untranslatable `error!`/`inspect_err`/`and_then` over `static mut`
state). They are TCB-allowed and call the tokenless shim versions; this boundary is
expected and acceptable.

## Status

`make verify-kernel MODULE=mm::phys` -> **31 verified, 0 errors**,
`assume=0 external_body=23 admit=0 trusted=0`.
`make verify` (all modules) -> **0 errors**, no regressions.
`./z build` (dual compilation) -> **OK**.
