# Final Review — `bump_allocator` (independent, skeptical)

Reviewer: independent agent. Every claim below was re-verified with tools
(grep, the AST/drift scripts, `make verify-bump-allocator`, vstd source search,
and line-by-line reading of `lib.rs` / `lib.spec.rs` / `lib.proof.rs`). Prior
summaries were not trusted.

Scope (in-scope functions only): `FixedSizeBumpAllocator::alloc_as`,
`FixedSizeBumpAllocator::alloc`, `align_up`, `BssStorage::as_mut_ptr`
(`BackendA/B/C::as_mut_ptr`). Out-of-scope (`fmt`, `new`, `default`) were not
flagged for missing specs.

Repo: `/home/ruize/nanvix-phy-specs` · branch `verus-ai-prove` · HEAD `bb63e624b`.

---

## Spec Quality

### `align_up` (external-top, fully verified — body checked)
- Contract: `ensures match result { Some(r) => align_up_spec(value, alignment) == Some(r), None => align_up_spec(value, alignment) is None }`.
- `align_up_spec` is a concrete `open spec fn`: `None` iff `alignment == 0` or the
  ceiling multiple exceeds `usize::MAX`; otherwise `Some(ceil(value/alignment) * alignment)`.
- Bidirectional (both `Some` and `None` arms tied to the same spec function), total,
  no overflow in spec (uses `int`/`nat`). The "already-aligned ⇒ result == value" and
  "least multiple ≥ value" caller properties are implied by the formula.
- **Verdict: good, complete, caller-usable.** This is the one in-scope function whose
  body is actually verified (not `external_body`).

### `assume_specification [<usize>::div_ceil]` (external-bottom)
- `requires y != 0` (matches the documented zero-divisor panic);
  `ensures result as int == (x + y - 1) / y` (the unsigned ceiling division, in `int`,
  no overflow). Faithful to the std API doc for unsigned `div_ceil`.
- **vstd was searched and genuinely lacks a `div_ceil` spec** (`grep -rn div_ceil`
  over `/home/ruize/toolchain/vstd` → 0 hits). This is a legitimate std-library
  external-bottom boundary per the spec-design audit checklist. **Acceptable.**

### `as_mut_ptr` (`BssStorage` trait method)
- `ensures result as int == base_of::<Self>()`. `base_of` is `uninterp` (a static's
  address is opaque to Verus). Pinning every call's result to the same ghost constant
  encodes **pointer stability** (same value each call) and binds the exec pointer to
  `BumpView::base`. The "≥ STORAGE_SIZE writable, A-aligned, exclusively-owned bytes"
  duties are the `unsafe trait` TCB contract, consumed as an assumption — correct
  placement. **Acceptable.**

### `alloc` / `alloc_as` (external-top, but `external_body` ⇒ assumed contracts)
Implemented contract (`lib.rs`):
- `requires bump_view(self).inv()`.
- `Ok(slot)`: with `a = slot_ref_addr(slot)`, `v = bump_view(self)` —
  `a % v.unit_align == 0` (alignment), `v.base <= a`, `a + N <= v.base + v.storage_size`
  (in-bounds). `alloc_as` additionally: `size_of::<T>() == N`, `align_of::<T>() <= A`.
- `Err(SizeMismatch) => size_of::<T>() != N`, `Err(AlignmentMismatch) => align_of::<T>() > A`
  (`alloc_as` only, bidirectional), and `Err(_) => true`.

Assessment:
- The size/alignment guard arms on `alloc_as` are bidirectional and genuinely useful —
  they make the `T`-vs-`(N,A)` cast contract caller-visible. **Good.**
- `slot_ref_addr` is `uninterp` (a `&mut T` carries no spec-readable address; mirrors
  `raw-array`). Because `alloc`/`alloc_as` are `external_body`, the alignment/in-bounds
  clauses are **assumed axioms** about the returned slot's abstract address. They are
  meaningful (a caller can use "the returned slot is A-aligned and within
  `[base, base+storage_size−N]`") and are NOT tautological. **Acceptable as trusted facts.**
- **The two `Err(_) => true` arms are tautological**, but acceptable per `caller_analysis.md`:
  every non-guard error (`Exhausted`/`Overflow`/`OutOfBounds`/`Misaligned`) is mapped by
  the kernel to a single `Error::OutOfMemory` and logged via `Display`; callers do not
  branch on the distinction. The bidirectional guard arms that callers DO rely on are
  specified.
- **`uninterp` usage** (`bump_view`, `slot_ref_addr`, `base_of`) is the *mechanical
  consequence* of `alloc`/`alloc_as` being `external_body` (TCB-approved) and of slot
  addresses being non-spec-readable — exactly the documented `raw-array` exception in the
  spec-design skill. It is **not** combined with `external_body` proof axioms to inject
  properties (there are no proof axioms), so it is not a `uninterp`-as-escape violation.

### Material weakening vs. the designed contract (the core finding)
`view_design.md §5` designs a much stronger contract for `alloc`/`alloc_as` that the
implemented contract **drops**. Designed (and explicitly justified as "what the kernel's
`unsafe` soundness depends on", §5.1 note) but **absent from `lib.rs`**:
1. `slot as int == v.slot_addr(v.allocated)` — the returned slot bound to the abstract
   slot geometry (the implemented spec instead uses a free `uninterp slot_ref_addr` with
   **no** connection to `v.slot_addr`).
2. `forall|j| 0 <= j < v.allocated ==> slot != v.slot_addr(j)` — **uniqueness / non-aliasing**.
3. `v.has_capacity()` + `v'.allocated == v.allocated + 1` on `Ok` — the allocation transition.
4. `Err(Exhausted) => !v.has_capacity()` — the Exhausted/monotone-capacity boundary.
5. `Err(_) => v'.allocated == v.allocated` — **no spurious consumption** on error.

The lemmas that *prove* these abstract facts (`lemma_geometry`,
`lemma_exhausted_boundary`, `lemma_alloc_transition`) exist and verify, but they are
**floating / orphan**: `grep` confirms none of `slot_addr`, `geometry_ok`, `spec_alloc`,
`has_capacity`, `is_consumed`, or `lemma_*` is referenced by any exec contract in
`lib.rs`. They are proven over `BumpView` and connect to nothing a caller invokes. This
is spec-design anti-pattern #5 (floating specs) and means the central safety property is
not delivered to callers. See Caller Coverage.

---

## Caller Coverage (3/6 + missing list)

Canonical list = the six "Key Invariants (caller perspective)" in `caller_analysis.md`.
"Covered" = surfaced on an in-scope exec function's `requires`/`ensures`.

| # | Caller invariant | On exec contract? | Evidence |
|---|------------------|-------------------|----------|
| 1 | Uniqueness / non-aliasing | **MISSING** | Not in `alloc`/`alloc_as` ensures; only in floating `lemma_geometry`/`lemma_alloc_transition` over `BumpView` |
| 2 | In-bounds | Covered | `alloc`/`alloc_as` `Ok`: `base <= a && a + N <= base + storage_size` |
| 3 | Alignment | Covered | `alloc`/`alloc_as` `Ok`: `a % unit_align == 0` (+ `align_of::<T>() <= A` on `alloc_as`) |
| 4 | Monotone capacity / Exhausted boundary | **MISSING** | `Err(_) => true`; no `has_capacity`/`allocated` on `Ok`; only in floating `lemma_exhausted_boundary` |
| 5 | Stable size contract | Covered | `alloc_as` bidirectional `SizeMismatch`/`AlignmentMismatch` + `Ok` size/align facts |
| 6 | No spurious consumption on error | **MISSING** | No `v'.allocated == v.allocated` on any error arm (designed in `view_design §5`, dropped) |

Per-function expectations also covered: `align_up` totality/None-conditions (full);
`as_mut_ptr` base-pinning/stability (full).

**Covered: 3/6 key invariants** (In-bounds, Alignment, Stable-size-contract).
**Missing: 3/6** — Uniqueness/non-aliasing, Monotone-capacity/Exhausted, No-spurious-consumption.

Context: `view_design.md §7` defers the `v → v'` transition *mechanism* to a
proving-phase atomic-ghost token. That legitimately defers #3/#4/#6 and the
`slot == slot_addr(allocated)` binding (they need to read `allocated`). But the
consequence is that, **as it stands, the verification does not establish the foundational
non-aliasing guarantee the kernel's `unsafe` page-table code relies on** — it is neither
on the exec contract nor reachable by callers through the lemmas. For a *final* review
this is an uncovered caller expectation, not merely a stylistic gap.

---

## Proof Completeness

- **`admit()` count: 0.** `grep -rnE "\b(admit)\b"` finds exactly one hit —
  `lib.proof.rs:6`, inside a header *comment* ("Bodies are `admit()` placeholders"),
  not an `admit()` expression. All three lemma bodies are real proofs (or trivially
  closed): `lemma_geometry` (full nonlinear/div-mod proof), `lemma_exhausted_boundary`
  (empty body — discharged directly from `inv()`), `lemma_alloc_transition`
  (`=~=` struct-update proof). Verus accepts them with 0 errors and `admit=0`.
- **`external_body` count: 2.** Locations:
  - `lib.rs:271` — `FixedSizeBumpAllocator::alloc`
  - `lib.rs:348` — `FixedSizeBumpAllocator::alloc_as`
  No other file contains `external_body`.

---

## TCB Compliance

Both `external_body` functions are listed in `verus-ai-logs/tcb-allowed.md`:
- `…/lib.rs::FixedSizeBumpAllocator::alloc` (lines 16–20) ✅
- `…/lib.rs::FixedSizeBumpAllocator::alloc_as` (lines 21–23) ✅

No `external_body` exists outside the allowed list. **Compliant.**

---

## Guardrails Compliance

`admit: 0, assume: 0, external_body: 2, assume_specification: 1, cfg-gated exec: 0`

- `assume(...)` (verification escape): 0 — `grep` for `assume\s*(` excluding
  `assume_specification` → none. Verus cheating-check independently reports `assume=0`.
- `assume_specification: 1` — `lib.spec.rs:28` (`<usize>::div_ceil`), justified
  external-bottom (vstd lacks it).
- `cfg-gated exec: 0` — only `#[cfg(verus_keep_ghost)]` (lib.rs:101,105 guarding the
  spec/proof `include!`s) and `#[cfg(test)]` (lib.rs:392) exist; neither gates a real
  exec branch/expression/match-arm. Verus cheating-check reports `cfg_gate=0`.

---

## AST Consistency: **PASS**

`python3 .../ast_consistency.py src/libs/bump_allocator/src/lib.rs` → exit 0,
"✅ All exec functions consistent", 12/12 functions MATCH, 0 mismatched, 0 missing,
0 extra (baseline auto-extracted from git). There are **no `// VERUS REWRITE` comments**
in the module (`grep` → none), so there is nothing to check for semantic equivalence;
exec code is byte-for-byte unchanged after annotation stripping.

---

## Verification: **PASS (0 errors)**

`make verify-bump-allocator` → **Exit code 0**, "cached (no recompilation)",
`cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
The pipeline tags the run `CHEATING_DETECTED`, but that flag is driven solely by
`external_body=2`, and both are TCB-approved — it is not a real cheating finding.
Function coverage `3/6` reflects the intentionally out-of-scope `fmt`/`new`/`default`.

**Spec-drift** (`spec_drift.py git-diff … --before HEAD`): exit 0, "✅ No contract drift
detected", 0 functions changed. Note this is trivially clean because the working tree
equals HEAD (the specs are already committed); the check cannot detect divergence from
the *design intent* in `view_design.md`, which is where the weakening (above) occurred.

---

## Bug Summary

`bugs.md` records "No code bugs found" for the in-scope functions, with rationale
(`align_up` total/guarded; `alloc` uses `checked_add`/`checked_mul` and validates
bounds/alignment; `alloc_as` validates size/align before touching storage).

- Reconciliation: I re-read all four in-scope functions. **The bugs.md assessment is
  still valid** — no true bug or context-dependent bug in the exec code. Every address
  computation is `checked_*`; bounds and alignment are validated before the slot is
  handed out; `align_up` guards `alignment == 0`. No new code bugs found.
- No surviving *verification failure* needs bug classification (verus passes with 0
  errors). The missing caller coverage (above) is a **spec-completeness gap**, not a code
  bug — the code is correct; the contract simply does not expose uniqueness/transition.
- **Stale documentation confirmed:** the `lib.proof.rs` header comment (lines 6–7),
  "Bodies are `admit()` placeholders during the specification phase; the proving phase
  discharges them," is **STALE** — the lemma bodies are actually proven now
  (`admit()` count = 0; verus passes). Minor; flagged below (not auto-fixed, to keep this
  review non-mutating of the artifact under review).

---

## Issues (priority ordered)

1. **[BLOCKER for PASS] Uniqueness / non-aliasing not on the exec contract.** The
   kernel's `unsafe` soundness depends on every returned slot being distinct, yet
   `alloc`/`alloc_as` ensure only alignment + in-bounds over an `uninterp slot_ref_addr`
   with no `slot == slot_addr(allocated)` binding and no distinctness clause. The proving
   facts live only in floating lemmas. Designed in `view_design §5.1` but dropped.
2. **[BLOCKER for PASS] Exhausted/monotone-capacity boundary and no-spurious-consumption
   not on the exec contract.** `Err(_) => true` on both functions; no `has_capacity`/
   `allocated`/transition surfaced. Designed in `view_design §5`, dropped.
3. **[Major] Floating/orphan lemmas.** `lemma_geometry`, `lemma_exhausted_boundary`,
   `lemma_alloc_transition` (and helpers `slot_addr`/`geometry_ok`/`spec_alloc`/
   `has_capacity`/`is_consumed`) connect to no exec contract — spec-design anti-pattern
   #5. They prove a *design model*, not the shipped API. Resolving #1/#2 (via the
   deferred atomic-ghost token attaching `BumpView` as the allocator's View) is what
   would bind them to exec.
4. **[Minor] Stale `lib.proof.rs` header comment** claims the lemma bodies are `admit()`
   placeholders; they are proven. Update the comment.
5. **[Informational] `Err(_) => true` arms are tautological** — acceptable here per
   `caller_analysis.md` (all collapse to `OutOfMemory`), but worth noting they carry no
   state-preservation guarantee.

---

## Result: **FAIL**

Mechanical guardrails all pass — **admit 0, assume 0, both `external_body` in the TCB,
AST PASS, verus 0 errors, spec-drift clean.** However, the explicit PASS bar also
requires *all caller expectations covered*, and **3 of the 6 documented key caller
invariants — Uniqueness/non-aliasing, Monotone-capacity/Exhausted boundary, and
No-spurious-consumption — are not covered by any in-scope exec contract.** They were
designed in `view_design.md §5` and dropped from `lib.rs`; the lemmas that establish them
float disconnected from the API. Most critically, the foundational non-aliasing property
the kernel's `unsafe` page-table code relies on is not delivered to callers. Per the
stated criteria this is a **FAIL** (caller coverage incomplete), pending the deferred
atomic-ghost / `PointsTo` token that would attach `BumpView` as the allocator's View and
wire the transition/uniqueness clauses (and the supporting lemmas) into the exec contract.
