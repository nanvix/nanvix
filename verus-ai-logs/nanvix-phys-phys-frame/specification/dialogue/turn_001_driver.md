## Turn 1: Full checklist sweep + first FAIL (No tautological ensures)

### Progress
- Done (PASS): coverage, caller-coverage, view-consistency, no-subsumed-ensures,
  no-internal-assume_specification, vstd-before-assume_spec, specs-for-caller,
  trait-obligations, loop-invariants, spec-drift (no weakening),
  cross-module-regression, verification/build.
- Current (FAIL, being addressed): **No tautological ensures** (== "Error paths
  have meaningful ensures").
- Remaining (reported, deferred): **No cheating on module's own functions**
  (external_body/admit counts — expected at this phase, must be discharged in the
  proof phase); spec-completeness (advisory); bug-awareness (advisory).

---

### Verification (commands run + findings)

**`make verify-kernel`** → exit 0. All 5 `mm::phys` modules verified
(`mm::phys`, `frame`, `kframe`, `manager`, `upool`). Summary line:
`cheating: assume=0 external_body=22 admit=7 ... coverage: 39/1022`. Cross-module
regression therefore PASSES (no regressions; this is the cached, green tree).

**Coverage — PASS.** `fn_coverage.py /tmp/frame_base.rs frame.rs`: 11/11 exec fns
matched, 0 missing/extra. Manual read confirms every in-scope function in
`verification-plan.json` carries a `#[verus_spec]`: the 8 `Inner::*` methods,
plus `instance`, `init`, `alloc`, `alloc_contiguous`, `free_count`, `free`,
`is_covered`, `book`, `alloc_range`, `share`, `refcount`.

**Caller coverage — PASS (with the Err note below).** Read `caller_analysis.md`.
Each documented caller expectation maps to a shim contract over `phys_view()`:
alloc/book→`allocated_frames.contains`+`refcounts==1`; share→still allocated;
refcount→exact value; is_covered→`covered().contains`; free→`inv()`+Drop-safety;
free_count→`free_frames.finite()`+`.len()`; alloc_range→region frames reserved.
The only gap is the dropped **error-path** facts (item 4 below).

**View consistency — PASS.** Specs reference only `FrameAllocView`
fields (`allocated_frames`/`free_frames`/`refcounts`) and `PhysMemView`
(`initialized`/`frames`/`covered()`/`region_frames()`), matching `view_design.md`.
Every shim/method `ensures inv()` (`phys_view().inv()` or `final(self).inv()`).
`Inner::view()` stays `closed`; the bitmap↔address mapping is confined to
`internal_inv`/`view()`. Consistent.

**No subsumed ensures — PASS (noted).** The Ok arms restate
`refcounts.contains_key(f)` alongside `allocated_frames.contains(f)`; under `wf`
these are equivalent, but they are deliberate trigger anchors and the load-bearing
`refcounts[f] == 1` / `== count` facts are NOT subsumed. Acceptable.

**No assume_specification (internal) — PASS.** `grep` across `mm/phys/`: the only
`assume_specification` is `kframe.spec.rs:33` on `crate::hal::mem::Address` (an
external dependency / trait), which the checklist explicitly permits. `frame.rs`
has none. `vstd-before-assume_spec` is therefore N/A for this module.

**Trait obligations — PASS.** `free` shim carries `opens_invariants none` +
`no_unwind` and no `requires`, satisfying the `Drop for UserFrame` /
`Drop for KernelFrame` contract documented in the caller analysis.

**Loop invariants — PASS.** The only `for` loops live in `Inner::alloc_contiguous`,
`Inner::alloc_range`, and `init` — all `external_body`, so no verified loop exists
that lacks an `invariant`. (Re-check after external_body removal in the proof phase.)

**Spec drift / no specs weakened — PASS.**
`spec_drift.py git-diff frame.rs --before <spec-phase-start 7c792c75>`:
`Ensures removed: 0`. Drift is limited to 3 previously-unspecced shims
(`alloc_contiguous`, `free_count`, `instance`) gaining requires+ensures — i.e.
new specs, not weakened ones. The added `requires` (`count > 0`,
`phys_view().initialized`, `phys_view().inv()`) match caller-supplied
preconditions. No original guarantee weakened.

**No cheating on module's own functions — FAIL (raw count), EXPECTED at this
phase, DEFERRED to proof phase.** From `cheating-detail.txt`, `frame.rs` owns:
- `external_body` (10): `Inner::alloc` (137), `Inner::alloc_contiguous` (210),
  `Inner::free` (290), `Inner::share` (368), `Inner::refcount` (428),
  `Inner::book` (481), `Inner::is_covered` (517), `Inner::alloc_range` (565),
  `instance` (652), `init` (689).
- `admit()` (7): shims `alloc` (756), `alloc_contiguous` (797), `free_count`
  (826), `free` (850), `book` (895), `alloc_range` (919), `share` (946).

These make the written specs currently *unproven*. That is the deliverable of the
**proof** phase (strip `external_body` + add body proofs; remove `admit()` by
proving the shim's post-state from `instance()`'s contract). `instance`/`init`
are TCB trust boundaries per `tcb-allowed.md`. This item is reported now and MUST
be re-audited to zero (for non-TCB functions) before the proof phase signs off; it
is not a specification-phase blocker on its own.

---

### Fix Request — Item 4/6: replace tautological `Err(_) => true`

`grep -n "Err(_) => true" frame.rs` → lines **686, 748, 792, 890, 914, 941**.
The checklist names `Err(_) => true` as the canonical tautological ensures. Three
of these drop **caller-relevant** error facts that the matching `Inner::*`
contract already establishes and that `caller_analysis.md` lists as relied upon.
On every Err path `final(self)@ == old(self)@ == phys_view().frames`, so each fact
below is expressible over the fixed `phys_view()` (the `admit()` discharges it).

Change these shim `ensures` Err arms:

1. **`book` (line 890).** `Inner::book` Err ⇒ `!old(self)@.free_frames.contains(phys_addr@)`.
   Caller (`book_mmio_regions`) relies on "allocator unchanged". Replace with:
   ```
   Err(_) => !phys_view().frames.free_frames.contains(phys_addr@),
   ```

2. **`alloc_range` (line 914).** `Inner::alloc_range` Err ⇒
   `!frames.subset_of(old(self)@.free_frames)`. Caller
   (`book_physical_memory_regions`) relies on "region was not fully free".
   Replace with (reusing the shim's existing `region_frames` vocabulary):
   ```
   Err(_) => !PhysMemView::region_frames(region@.start, region@.size)
       .subset_of(phys_view().frames.free_frames),
   ```

3. **`share` (line 941).** `Inner::share` Err ⇒ frame not allocated OR refcount
   would overflow. Caller (`UserFrame::share`) relies on exactly this. Replace with:
   ```
   Err(_) => !phys_view().frames.allocated_frames.contains(frame@)
       || (phys_view().frames.refcounts.contains_key(frame@)
           && phys_view().frames.refcounts[frame@] >= 255),
   ```

4. **`alloc` (line 748).** `Inner::alloc` Err ⇒ `old(self)@.free_frames.is_empty()`
   (out-of-memory witness). Strengthen to:
   ```
   Err(_) => phys_view().frames.free_frames.is_empty(),
   ```

For **`alloc_contiguous` (792)** and **`init` (686)**: `Inner::alloc_contiguous`
Err states only `final == old` (nothing expressible without `old(phys_view())`),
and `init` Err has no meaningful post-state beyond the already-stated
`phys_view().inv()`. If you keep `Err(_) => true` for these two, you must prove it
is genuinely inexpressible by quoting the corresponding `Inner` contract Err arm —
a prose justification is not acceptable.

**How to verify the fix:** after editing, run
```
grep -n "Err(_) => true" src/kernel/src/mm/phys/frame.rs   # only 686/792 may remain, with justification
make verify-kernel                                          # must stay exit 0
```
Do not weaken any other arm to make it pass; the `admit()` already discharges the
proof obligation at this phase.

### Status
- FAIL items open: **No tautological ensures** (this fix request);
  **No cheating on own functions** (deferred to proof phase, reported).
- No STOP file written — open FAIL items remain.
