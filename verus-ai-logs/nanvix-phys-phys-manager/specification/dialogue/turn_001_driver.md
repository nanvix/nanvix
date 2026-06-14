## Turn 1: Full checklist walkthrough + first FAIL (Caller coverage / watermark policy)

### Progress
- Done (PASS): fn-coverage, view-consistency, no-subsumed-ensures, no-assume_specification,
  vstd-searched, trait-obligations, loop-invariants, no-cheating-own-fns,
  no-specs-weakened (vs precedent), bug-awareness, cross-module-regression, verification+build.
- Note (technically present, but matches accepted precedent / unstrengthenable):
  no-tautological-ensures, error-paths-meaningful.
- Current (FAIL): **Caller coverage / Specs-usable-by-caller / Spec-completeness** —
  the kernel-watermark policy split is absent from the user-allocation contracts.
- Remaining: re-verify after fix.

---

### Verification (commands run + evidence)

**`make verify-kernel`** → Exit code 0. All modules verify:
`mm::phys`, `mm::phys::frame`, `mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool`.
Cheating check: `assume=0 external_body=22 admit=2 trusted=0`. Cross-module regression PASS.

**Cheating detail** (`verus-logs/cheating-detail.txt`) — manager-owned items:
- `manager.rs` `external_body` ×6: `init`, `alloc_many_user_frames`, `alloc_user_frame`,
  `check_user_watermark`, `alloc_kernel_frame`, `alloc_many_kernel_frames`.
  All six are explicitly enumerated in `verus-ai-logs/tcb-allowed.md` (stateless-facade
  rationale, mirrors the `frame.rs` free-function shims). → PASS for this phase.
- `manager.proof.rs` `admit` ×2: `lemma_watermark_monotone`, `lemma_contiguous_run_distinct`.
  Proof file header states bodies are `admit()` in the spec-design phase, discharged in the
  proving phase. → acceptable for the specification phase.

**`fn_coverage` (in-scope):** the 6 verification-order targets each carry `#[verus_spec]`
with `requires`/`ensures`. `get_mut` is on the tcb-allowed skip list (caller_analysis.md
§"Script-output corrections"), not a target. → PASS.

**`grep assume_specification src/kernel/src/mm/phys/`** → none. → PASS.

**View consistency:** specs are stated over `phys_view()` / `FrameAllocView` and re-establish
`phys_view().inv()` + `phys_view().initialized`. This deviates from the `self@`-View in
`view_design.md`, but the deviation is documented (manager.spec.rs header + view_design.md
implementation note: a field-less facade's `view(&self)` is constant, so it cannot witness
global mutation; there is no `old(phys_view())`). The same monotone style is used by the
already-accepted `frame::book` / `frame::alloc_range` shims (verified). → PASS.

**No-tautological / Error-paths (NOTE, not actionable):** `alloc_user_frame`,
`alloc_kernel_frame`, and `check_user_watermark` use `Err(_) => true`. This is literally the
checklist's bad example. However: (a) the sibling shims `frame::book` and `frame::alloc_range`
(frame.rs:776, 821) use the identical `Err(_) => true`; (b) strengthening the Err arm of a
`phys_view()`-style shim requires `old(phys_view())`, which Verus cannot express for a nullary
`spec fn`. The bulk fns already carry a meaningful Err arm (`final(frames)@.len() == 0`).
I am therefore NOT requesting a change here — it is consistent with established module
precedent and unstrengthenable. (If the fixer disagrees, show tool output proving a stronger
Err arm type-checks.)

---

### FAIL — Caller coverage: the watermark policy split is missing from the user-alloc specs

**Evidence (byte-identical ensures):**
`alloc_user_frame` (manager.rs:253-262) and `alloc_kernel_frame` (manager.rs:338-347) have
**character-for-character identical** `ensures`:
```
ensures
    phys_view().inv(),
    phys_view().initialized,
    match result {
        Ok(frame) => {
            &&& phys_view().frames.allocated_frames.contains(frame@)
            &&& frame@ % spec_page_size() == 0
        },
        Err(_) => true,
    },
```
The same holds for `alloc_many_user_frames` vs `alloc_many_kernel_frames`: they differ ONLY by
the contiguity conjunct, not by any watermark fact.

**Why this is a defect (not a style nit):**
- `caller_analysis.md` lists the watermark as a KEY invariant and states explicitly, for
  `alloc_user_frame`: *"Would break callers: skipping the watermark check (the doc/comment at
  the call site explicitly relies on this single-frame helper keeping the watermark gate)."*
- `view_design.md` mapping table requires `alloc_user_frame` success to depend on
  `spec_watermark_ok(self@, 1)` and explicitly specifies the kernel variants **without** it.
- The entire reason `check_user_watermark` exists is the user/kernel policy split. Yet a caller
  reading `alloc_user_frame`'s contract cannot conclude the allocation respected
  `KERNEL_WATERMARK`. The user contract is presently indistinguishable from the
  watermark-bypassing kernel contract. The load-bearing guarantee is invisible.
- This is statable **without** `old(phys_view())`: `check_user_watermark(count)` guarantees
  `free_count >= KERNEL_WATERMARK + count` *before* servicing; after handing out `count` frames,
  `free' = free - count >= KERNEL_WATERMARK`, i.e. `spec_watermark_ok(phys_view().frames, 0)`.

### Fix Request

Strengthen the **user** allocation contracts (and only those) so the watermark policy is
caller-observable. Leave the **kernel** contracts unchanged (they bypass the watermark).

1. In `manager.rs::alloc_user_frame` `ensures`, add to the `Ok` arm (and add the finiteness
   fact the watermark predicate needs, since `PhysMemView::inv` does NOT imply finiteness —
   `mod.spec.rs:105` `inv := initialized ==> frames.wf()`, and `FrameAllocView::wf` has no
   `finite()` conjunct):
   ```
   Ok(frame) => {
       &&& phys_view().frames.allocated_frames.contains(frame@)
       &&& frame@ % spec_page_size() == 0
       &&& phys_view().frames.free_frames.finite()
       &&& spec_watermark_ok(phys_view().frames, 0)
   },
   ```

2. In `manager.rs::alloc_many_user_frames` `ensures`, add the same two conjuncts
   (`free_frames.finite()` and `spec_watermark_ok(phys_view().frames, 0)`) to its `Ok(())` arm.

3. Do **NOT** add these to `alloc_kernel_frame` or `alloc_many_kernel_frames` — the kernel
   variants must remain watermark-free so their contracts stay distinct (and correct: kernel
   allocation may legitimately drive free frames below the watermark).

**Verify the fix with:**
- `make verify-kernel` → must remain Exit 0 (all modules, no new errors).
- `diff` the two single-frame `ensures` blocks: they must NO LONGER be identical
  (`sed -n '253,262p'` vs `sed -n '338,347p' src/kernel/src/mm/phys/manager.rs`).
- Confirm `spec_watermark_ok` and `phys_view` are in scope in `manager.rs` (they are used
  already by `check_user_watermark`, so no new imports needed).

Justification is not a fix: if you believe the watermark cannot be stated in the post-state,
show the Verus error from attempting conjunct (3) above — otherwise change the code.
