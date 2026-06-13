## Turn 1: Every in-scope exec function has requires/ensures

### Progress
- Done (verified PASS this turn): #7 (no workspace-internal assume_specification),
  #8 (vstd searched first), #14 (no specs weakened / clean slate), #15 (bug awareness).
- Current (FAIL, primary): #1 in-scope exec functions missing requires/ensures.
- Remaining / blocked-by-#1: #2 caller coverage, #3 view consistency, #6 error-path
  ensures, #9 caller-usable specs, #10 trait obligations, #12 loop invariants.
- Reported (not blocking the spec phase, carried to proving): #13 admit counts.
- Not runnable this turn (template command placeholders unfilled): #5 subsumed,
  #11 spec-completeness skill, #16 cross-module regression `{verus_cmd_all}`,
  the `{spec_drift_cmd}` for #14.

### Verification

Ran the authoritative tool:

```
make verify-bump-allocator
  Exit code : 0
  cheating  : assume=0 external_body=0 admit=3 trusted=0 no_decreases=0 cfg_gate=0
  coverage  : 1/6 exec functions have contracts
  status    : CHEATING_DETECTED
```

`coverage-unverified.txt` lists the 5 functions WITHOUT contracts:
`fmt, new, alloc, alloc_as, default`.

Grep of `lib.rs` for `verus_spec|requires|ensures`:
- line 126–127: `align_up` — has `#[verus_spec(... ensures ...)]`. ✅
- line 198: `as_mut_ptr` — has a **bare** `#[verus_spec]` with **no ensures**. ❌
- `alloc` (line 260) — **no spec at all**. ❌
- `alloc_as` (line 318) — **no spec at all**. ❌

Per `caller_analysis.md` (lines 143–145) the in-scope functions are:
`alloc_as`, `alloc`, `align_up`, `as_mut_ptr`. Only `align_up` is specified.
`new`/`default`/`fmt` are out of scope, so the meaningful denominator is the four
in-scope functions: **1/4 specified**.

`view_design.md` already fully designs the missing contracts:
- §5.1 — full `requires v.inv()` / `ensures` (Ok/Err arms, config-unchanged,
  slot_addr, alignment, in-bounds, distinctness) for `alloc`.
- §5.2 — full match-style ensures (SizeMismatch / AlignmentMismatch / Ok / Err)
  for `alloc_as`.
- §4.2 — backend spec for `as_mut_ptr` (returns `base`, `base % A == 0`, stability).

These designs exist only as prose plus standalone `BumpView` proof lemmas in
`lib.proof.rs`; they are **not attached** to the exec functions, so no caller can
use them. This is the central deliverable of the specification phase and it is
missing. **FAIL.**

#### Per-item determinations (full walk-through)

- #1 requires/ensures coverage — **FAIL** (1/4 in-scope; `alloc`, `alloc_as`,
  `as_mut_ptr` unspecified). Primary fix below.
- #2 caller coverage — **FAIL (consequent)**. `caller_analysis.md` expectations
  (unique/in-bounds/aligned/`'static` slots; `Exhausted` boundary; size/align
  guards; `as_mut_ptr` stability) are not encoded as ensures on the functions.
- #3 view consistency — **FAIL (consequent)**. `BumpView` + `inv()` exist and are
  internally consistent, but no exec spec references `self.view()`/`inv()`; the
  View is explicitly *not yet attached* (`lib.spec.rs` lines 12–17, `view_design`
  §2 deferral).
- #4 no tautological ensures — **PASS (for what exists)**. `align_up` and the proof
  lemmas are non-tautological. Watch the future `Err(_)` arms.
- #5 no subsumed ensures — **N/A** for what exists; re-check after specs added.
- #6 error-path ensures — **FAIL (consequent)**. `alloc`/`alloc_as` have no error
  ensures at all yet (design §5 has them; not implemented).
- #7 no workspace-internal assume_specification — **PASS**. Only
  `assume_specification [ <usize>::div_ceil ]` (std/external).
- #8 vstd searched first — **PASS**. Grep of the Verus toolchain found no vstd spec
  for `div_ceil`; the `div_ceil` assume_specification is justified.
- #9 specs usable by caller — **FAIL (consequent)**. Missing `alloc`/`alloc_as`
  specs cannot be used in kernel proofs.
- #10 trait obligations — **FAIL**. `as_mut_ptr` `#[verus_spec]` is empty; the
  `BssStorage` semantic contract (stable, `A`-aligned, `>= STORAGE_SIZE` bytes) is
  not stated.
- #11 spec-completeness (advisory) — **incomplete**; cannot pass while specs are
  missing.
- #12 loop invariants — **FAIL/pending**. `alloc` has a CAS `loop` (lib.rs:262)
  with no `invariant` clause; Verus will reject once the fn is specified.
- #13 admit/assume/external_body/trusted — reported: `admit=3`
  (`lib.proof.rs:22 lemma_geometry`, `:39 lemma_exhausted_boundary`,
  `:54 lemma_alloc_transition`); assume=0, external_body=0, trusted=0. These are
  spec-phase placeholders to be discharged in the proving phase — acceptable now,
  **must not remain**. Not a spec-phase blocker by itself.
- #14 no specs weakened — **PASS / N/A**. `caller_analysis.md` (lines 131–139)
  records the prior `lib.spec.rs`/`lib.proof.rs` as empty `verus!{}` blocks (clean
  slate); specs were added, none weakened. (`{spec_drift_cmd}` unfilled.)
- #15 bug awareness — **PASS**. `bugs.md` present and reasoned.
- #16 cross-module regression — **NOT RUN** (`{verus_cmd_all}` unfilled). Current
  crate verifies (exit 0). Carry forward.
- #17 verification + build — verify: exit 0 but `CHEATING_DETECTED` + coverage 1/6;
  `{build_cmd}` unfilled. Not clean.

### Fix Request

Add the designed contracts to the three unspecified in-scope functions, exactly as
`view_design.md` specifies. Do **not** weaken the §5/§4.2 designs.

1. `as_mut_ptr` (lib.rs:198) — replace the bare `#[verus_spec]` with an `ensures`
   encoding `view_design.md` §4.2: the returned pointer-as-int equals the pool base
   ghost constant and is `A`-aligned (`result as int % (A as int) == 0`) and stable.
   This is the trait-method backend spec the allocator re-reads each call.

2. `alloc` (lib.rs:260) — add `requires`/`ensures` per `view_design.md` §5.1:
   `requires self.view().inv()`; ensures `v'.inv()`, configuration-unchanged, and
   the match arms — `Ok(slot)`: `v.has_capacity()`, `v'.allocated == v.allocated+1`,
   `slot as int == v.slot_addr(v.allocated)`, alignment, in-bounds, and the
   `forall|j| 0<=j<v.allocated ==> slot as int != v.slot_addr(j)` distinctness;
   `Err(Exhausted)`: `!v.has_capacity() && v'.allocated == v.allocated`;
   `Err(_)`: `v'.allocated == v.allocated`. The `Err(_)` arm must keep the
   `allocated`-unchanged fact (NOT `=> true`).

3. `alloc_as` (lib.rs:318) — add `requires`/`ensures` per `view_design.md` §5.2:
   the `SizeMismatch`/`AlignmentMismatch` guard arms (with `v'.allocated ==
   v.allocated`), the `Ok` arm mirroring `alloc` plus `size_of::<T>()==N &&
   align_of::<T>()<=A`, and the propagated `Err(e) => v'.allocated == v.allocated`.

4. To make the per-call `v -> v'` specs expressible, attach `BumpView` as the View
   of `FixedSizeBumpAllocator` (the `View`/atomic-ghost-token attachment that
   `view_design.md` §2 and `lib.spec.rs` lines 12–17 deferred). The spec phase needs
   a `self.view()` to refer to; use `vstd` atomic-ghost (`PAtomicUsize`/`PointsTo`)
   or equivalent so `allocated` is readable in spec. Pin `base/stride/unit_size/
   unit_align/capacity/storage_size` to the type constants in `inv()`.

5. `alloc`'s CAS `loop` (lib.rs:262) must get an `invariant` clause (Verus requires
   one). Define any non-trivial invariant as a `spec fn` in `lib.spec.rs`/
   `lib.proof.rs`.

Verify after changes:
- `make verify-bump-allocator` → exit 0, and the coverage line must show all four
  in-scope functions specified (target `coverage: >= 4/6`, ideally 6/6 with
  `new`/`default` if cheaply specifiable). Re-read
  `verus-ai-logs/verify-bump-allocator/verus-logs/coverage-unverified.txt` to
  confirm `alloc`, `alloc_as`, `as_mut_ptr` no longer appear.

If, after a genuine attempt, attaching the View / making `allocated` spec-readable
proves impossible without changing `BumpView`'s fields or `inv()`, report back with
the concrete Verus error so I can evaluate a view-design ROLLBACK. Do not paper over
it with `admit`/`assume` on the exec functions — justification is not a fix.
