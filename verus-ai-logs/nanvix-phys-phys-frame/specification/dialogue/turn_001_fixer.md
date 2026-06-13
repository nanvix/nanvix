## Response to Turn 1

### What I Changed

Lead items **#4 / #6** (non-meaningful `Err` arms) and **#7** (intra-crate
`assume_specification` recorded in TCB).

**`src/kernel/src/mm/phys/frame.rs`** — strengthened the error-path `ensures` of the
caller-facing free functions to mirror their `Inner` callees' post-conditions (which
the reviewer's analysis confirmed are expressible and caller-required). No `Ok` arm
weakened.

1. `alloc` (~line 747-757): `Err(_) => true` →
   `Err(_) => crate::mm::phys::phys_view().frames.free_frames.is_empty()`.
   Mirrors `Inner::alloc`'s `old(self)@.free_frames.is_empty()` (frame.rs:132) and
   unblocks `Upool::alloc`'s `Err(_) => old(self)@.free_count() == 0`.

2. `book` (~line 855-866): `Err(_) => true` →
   `Err(_) => !crate::mm::phys::phys_view().frames.free_frames.contains(phys_addr@)`.
   Mirrors `Inner::book`'s `!old(self)@.free_frames.contains(phys_addr@)` (frame.rs:493).

3. `alloc_range` (~line 873-884): `Err(_) => true` →
   `Err(_) => !phys_view().frames.all_free(region_frame_addrs(region@.start, region@.size))`.
   Mirrors `Inner::alloc_range`'s `!frames.subset_of(old(self)@.free_frames)`
   (frame.rs:578); `all_free(set)` is definitionally `set.subset_of(free_frames)`
   (mod.spec.rs:141). The `all_free` predicate already existed — no new spec fn added.

4. `alloc_contiguous` (~line 771-786): **`Err(_) => true` retained, with tool evidence.**
   `Inner::alloc_contiguous`'s only `Err` guarantee is `final(self)@ == old(self)@`
   (state-unchanged, frame.rs:208-210). At the free-function boundary the singleton has
   no spec-readable `old()` receiver — `phys_view()` is a fixed `uninterp spec fn`, so a
   post-state-only predicate cannot express "state unchanged". No `FrameAllocView`
   predicate over the post-state captures a non-trivial `Err` condition, and no caller
   consumes one (`caller_analysis.md`: the sole caller `manager.rs:449` "propagated with
   `?`", consumes no abstract `Err` fact). Evidence below.

**`verus-ai-logs/tcb-allowed.md`** — added a section "`assume_specification` for
not-yet-verified callees" recording the `frame.spec.rs` declarations per **#7**,
separating the genuinely-external `::arch::*` placeholders from the intra-crate
`crate::hal::mem::*` ones (`FrameAddress::{from,into}_frame_number`,
`PhysicalAddress::into_frame_number`, `TruncatedMemoryRegion::{start,size}`,
`PageAligned` `Address::into_raw_value` / `Deref::deref`), with the
"superseded when the address layer is verified" rationale.

#### Tool evidence for item 4 (`alloc_contiguous` Err arm stays `true`)

`Inner::alloc_contiguous` Err arm (frame.rs:208-210):
```
Err(_) => {
    final(self)@ == old(self)@
}
```
This is a relation between `old(self)@` and `final(self)@`. The free-function wrapper
specs are phrased over `crate::mm::phys::phys_view().frames` — a fixed value of an
`uninterp spec fn phys_view()`, with no `old()` form. Therefore no post-state-only
`FrameAllocView` predicate is equivalent to "state unchanged", and the only sound
caller-facing fact is `true`. The `Ok` arm (page-aligned base + non-overflow bound)
remains fully specified.

### Verification

`make verify-kernel MODULE=mm::phys` (fresh, after `touch frame.rs`):
```
verification: 47 verified, 0 errors (exit 0)
cheating: assume=0 external_body=14 admit=24 trusted=0 no_decreases=0 cfg_gate=17
status: CHEATING_DETECTED
```
(`CHEATING_DETECTED` reflects the spec-phase `admit()` placeholders, item #13 — not a
spec defect. Error count 0, counts otherwise unchanged from before the edit.)

`make check-kernel` (normal dual compilation):
```
{"reason":"build-finished","success":true}
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.56s
```

`spec_drift.py git-diff frame.rs --before 97e31408 --after HEAD`:
```
- Contract drift: 4   (alloc, alloc_range, book, is_covered)
- Ensures removed: 1   (alloc — whole match block textually replaced)
- Functions removed: 0
```
The single "ensures removed" is `alloc`'s match block replaced by a **strictly stronger**
one (Ok adds `allocated_frames.contains(frame@)`; Err goes `true` → `free_frames.is_empty()`)
— the explicitly-permitted "stronger replacement" case. `alloc_range`/`book`/`is_covered`
show only "requires added" (matching documented caller expectations) + "ensures added".
No guarantee weakened.

`fn_coverage.py frame.rs frame.rs`: `Matched 11 / 11, Missing 0, Extra 0` — unchanged.

### Result: FIXED

Items #4 and #6 resolved for `alloc`, `book`, `alloc_range` (meaningful `Err` arms
mirroring the `Inner` callees, unblocking the #2/#9 `Upool::alloc` fallout). Item #4's
`alloc_contiguous` `Err(_) => true` retained with tool evidence that no non-trivial
post-state `Err` fact is expressible at the singleton boundary or consumed by any caller.
Item #7 recorded in `tcb-allowed.md`. Items #12 (loop invariants) and #15 (alloc_range
off-by-one) are proving-phase debt already recorded in `bugs.md` and out of scope for
this specification turn.
