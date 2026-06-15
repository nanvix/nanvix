## Turn 1 (specification re-review after view-design re-run): View consistency / specs weakened → FAIL

> Context: the prior specification round rolled back to view-design (old
> `turn_001_driver.md`, 08:33). view-design re-ran and regenerated
> `view_design.md` (08:44) mandating a `tracked PhysAuth` carrier to make
> post-state effects expressible. This review checks whether the re-run
> specification phase actually implemented that design. **It did not.**
> No new rollback is requested — the View artifact is now correct; the gap is
> that the spec code was not realigned to it.

### Progress
- Done (PASS): items 1, 7, 8, 10, 12, 15, 16, 17.
- Current FAILs (one shared root cause): items 2, 3, 4, 6, 9, 13, 14 (+11 advisory).
- Remaining/secondary: item 5 (revisit after the redesign).

### Verification (commands run, not trusted verbally)

`make verify-kernel` → **exit 0** ("32 verified, 0 errors") but the summary is:

```
cheating: assume=0 external_body=22 admit=1 trusted=0
status: CHEATING_DETECTED
```

`verus-logs/cheating-detail.txt`, in-scope `mm/phys/frame.rs`: external_body at
alloc(137), alloc_contiguous(210), free(290), share(368), refcount(428),
book(481), is_covered(517), alloc_range(565), instance(652), init(689) = 10;
plus `admit` at frame.rs:846 (the `free` shim).

Files read in full: `frame.rs`, `frame.spec.rs`, `frame.proof.rs`, `mod.spec.rs`,
`view_design.md`, `caller_analysis.md`, `bugs.md`, `tcb-allowed.md`,
`rollback_specification_to_view-design_1.md`.

**The regenerated `view_design.md` mandates a `tracked PhysAuth`** that replaces
the 0-ary `phys_view()` constant so the mutating shims can name pre/post
(`old(auth)@` vs `auth@`) and carry strong contracts, e.g. `alloc`:

```
auth@ == old(auth)@.spec_alloc_one(frame@)
&& auth@.frames.allocated_frames.contains(frame@)
&& auth@.frames.refcounts[frame@] == 1
```

The code does NOT implement it:

- `grep PhysAuth src/` → **none**. `PhysAuth` was never added.
- `mod.spec.rs:171` still has `pub uninterp spec fn phys_view() -> PhysMemView;`
  (the rejected constant). `mod.spec.rs` mtime 02:27 — never touched after the
  08:44 view-design re-run.
- The verified reservation shims keep the **weakened pre-state** specs the
  rollback report condemned:
  - `alloc` (744-750): `Ok ⇒ free_frames.contains(frame@)`, `Err ⇒ true`.
  - `alloc_contiguous` (764-787): `Ok ⇒ {base+i·page}.subset_of(free_frames)`,
    `Err ⇒ true`.
  - `book` (883-898): `Ok ⇒ free_frames.contains(phys_addr@)`.
  - `alloc_range` (904-922): `Ok ⇒ region_frames.subset_of(free_frames)`.
  None state the post-state allocation effect callers require.
- The real guarantee is still relocated into `external_body` axioms NOT in
  `tcb-allowed.md`: `manager::alloc_user_frame` (manager.rs:249-267) asserts
  `Ok ⇒ phys_view().frames.allocated_frames.contains(frame@)` as an unproven
  axiom; same for `alloc_kernel_frame`, `alloc_many_user_frames`,
  `alloc_many_kernel_frames`. Reproducer `02_goal_is_false.rs` proves this fact
  is *provably false* over the constant `phys_view()`, so the axiom masks an
  unsoundness. `view_design.md` "Threading plan" says these must be **deleted**
  and the guarantee **derived** from the threaded shims.
- `bugs.md` re-frames the weakening as "intended… subsystem-wide redesign, out of
  scope". That contradicts the regenerated `view_design.md`, which puts the
  `PhysAuth` strengthening **in scope** for this phase. Per the review rules,
  justification is not a fix.

### Per-item verdicts
- [PASS] 1. Coverage: every in-scope shim + `Inner::*` has requires/ensures.
- [FAIL] 2. Caller coverage: `caller_analysis.md` requires `alloc` Ok ⇒ "now in
  `allocated_frames`, `refcounts[frame]==1`"; "Would break callers: a frame not
  in `allocated_frames`/refcount≠1". Shims deliver only pre-state. Same for
  `book`, `alloc_range`, `alloc_contiguous`.
- [FAIL] 3. View consistency: specs reference the 0-ary `phys_view()` constant;
  the mandated `PhysAuth` carrier is absent; the View's `spec_alloc_one/_set/
  _share` transitions go unused.
- [FAIL] 4. Tautological ensures: `alloc` Err⇒true (749), `alloc_contiguous`
  Err⇒true (786), `init` Err⇒true (686).
- [—] 5. Subsumed ensures: secondary; revisit after the redesign.
- [FAIL] 6. Meaningful error paths: `alloc`/`alloc_contiguous` shim Err arms carry
  no info (the `Inner::*` Err arms at 131/205 do — the shims dropped them).
- [PASS] 7. No assume_specification for workspace-internal code (assume=0).
- [PASS] 8. vstd/assume_specification: none used.
- [FAIL] 9. Specs usable in caller proofs: they are not — `manager::alloc_*` had
  to become `external_body` axioms precisely because the shim specs are too weak.
- [PASS] 10. Trait obligations: `free` honors the `Drop` contract
  (`opens_invariants none`, `no_unwind`, no `requires`).
- [—] 11. Spec completeness (advisory): the nondeterminism is *forced weakening*,
  not caller-acceptable intentional nondeterminism → FAIL advisory.
- [PASS] 12. Loop invariants: the only loops are inside `external_body` `Inner::*`;
  verified shims have none.
- [FAIL] 13. No cheating on module's own functions:
  - `admit` at frame.rs:846 (`free` shim) — on this module's own function.
  - `Inner::*` + `instance` + `init` external_body (10) are TCB-allowed
    (`tcb-allowed.md` §1–2: untranslatable `error!`/`arch` newtypes) — acceptable.
  - **Not acceptable:** `manager::alloc_user_frame`/`alloc_kernel_frame`/
    `alloc_many_user_frames`/`alloc_many_kernel_frames` `external_body` axioms hold
    the relocated strong guarantee; they are not in `tcb-allowed.md` and must be
    removed (guarantee derived from threaded shims).
- [FAIL] 14. No specs weakened: reservation shims still weakened from the
  documented post-state contracts to pre-state facts — the exact defect that
  triggered the prior rollback, now unaddressed after the view-design re-run.
- [PASS] 15. Bug awareness: `bugs.md` exists (but its "out of scope" conclusion is
  now stale vs the regenerated `view_design.md`).
- [PASS] 16. Cross-module regression: all `mm::phys` modules verify, exit 0 (passes
  only because the specs are weak).
- [PASS] 17. Verification compiles (exit 0) — but `status: CHEATING_DETECTED`.

### Fix Request (root cause — implement the already-designed `PhysAuth`; unblocks 2,3,4,6,9,11,13,14)

Realign the spec code to the regenerated `view_design.md` ("The Fix: a Diff-able
Mechanism"). This is now a *specification-phase* implementation task, not a
rollback.

1. `mod.spec.rs`: remove `pub uninterp spec fn phys_view() -> PhysMemView;`; add
   `pub tracked struct PhysAuth { ... }` with `spec fn view(self) -> PhysMemView`
   and `spec fn inv(self) -> bool { self.view().inv() }`. Add `spec_alloc_one`,
   `spec_alloc_set`, `spec_share`, `spec_free` on `PhysMemView` (keep
   `spec_book_frame`/`spec_book_frames` as aliases).

2. `frame.rs`: make `instance()` take `Tracked(&mut PhysAuth)` and bridge
   `(*r)@ == auth@.frames` with `auth@ == old(auth)@`. Thread
   `Tracked(&mut PhysAuth)` through the mutating shims and restore STRONG
   post-state contracts with **meaningful** Err arms (not `=> true`):
   - `alloc`: `Ok ⇒ frame.inv() && auth@ == old(auth)@.spec_alloc_one(frame@) &&
     allocated_frames.contains(frame@) && refcounts[frame@]==1`;
     `Err ⇒ auth@ == old(auth)@`.
   - `book`: `Ok ⇒ auth@ == old(auth)@.spec_alloc_one(phys_addr@) &&
     allocated_frames.contains(phys_addr@)`.
   - `alloc_range`: `Ok ⇒ auth@ == old(auth)@.spec_alloc_set(region_frames) &&
     region_frames.subset_of(allocated_frames)`.
   - `alloc_contiguous`: `Ok ⇒` the `count` page-strided frames are now allocated,
     each refcount 1 (via `spec_alloc_set`).
   - `share`: `Ok ⇒ auth@ == old(auth)@.spec_share(frame@)`.
   Query shims (`refcount`, `is_covered`, `free_count`) take `Tracked(&PhysAuth)`.
   `free` may keep the weak `opens_invariants none/no_unwind` Drop contract — the
   one caller-justified exception — but **remove the `admit()`**: a sound
   always-true Drop contract needs no admit; the per-reference transition is a
   proving-phase concern, not a spec-phase axiom.

3. `manager.rs`: delete the `external_body` on `alloc_user_frame`,
   `alloc_kernel_frame`, `alloc_many_user_frames`, `alloc_many_kernel_frames`;
   thread `&mut self.auth` so their guarantees are derived from the strengthened
   shims.

4. `bugs.md`: mark the "phys_view() is a constant → post-state inexpressible"
   limitation **resolved by `PhysAuth`**, not a permanent constraint.

Verify with `make verify-kernel`: it must report 0 errors, `admit=0` for
`frame.rs`, and `status` must NOT be `CHEATING_DETECTED` from the relocated
manager axioms; then a build (`make all-kernel`) to confirm exec compiles.
Do not reply with a justification for keeping the weakened specs — the
regenerated `view_design.md` makes the `PhysAuth` strengthening in scope.

**Verdict: FAIL.** STOP not created — items 2, 3, 4, 6, 9, 13, 14 unresolved.
