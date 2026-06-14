# Final Verification Review — `bump-allocator` (STRICT, INDEPENDENT)

- Reviewer: independent final verification pass
- Date: 2026-06-15
- Repo: `/home/ruize/nanvix-phy-specs-bottom-up`
- Base branch: `verus-ai-prove-bottom-up` (checked out)
- In-scope files:
  - `src/libs/bump_allocator/src/lib.rs` (exec + attribute specs)
  - `src/libs/bump_allocator/src/lib.spec.rs`
  - `src/libs/bump_allocator/src/lib.proof.rs`
- In-scope target functions: `FixedSizeBumpAllocator::alloc_as`,
  `FixedSizeBumpAllocator::alloc`, `align_up`, `BssStorage::as_mut_ptr`

---

## Mechanical gate summary (all clean)

| Gate | Result |
|------|--------|
| Verus verification | **12 verified, 0 errors** (exit 0) |
| `admit()` | **0** |
| `assume(...)` | **0** |
| `external_body` | **2** — both in `tcb-allowed.md` ✅ |
| `assume_specification` | **1** (`<usize>::div_ceil`) — std boundary, vstd has no spec ✅ |
| cfg-gated exec | **0** |
| AST consistency (vs base branch) | **PASS** |
| AST consistency (vs pre-verus `e79991a92`) | **PASS** |
| Spec drift | **0** (no contract drift) |
| Unit tests / doctest | **4/4 pass** |
| `cargo build` (crate) | **PASS** |

Despite all mechanical gates passing, this review returns **FAIL** on
**spec-quality grounds** (Task 1/2): the verified `alloc`/`alloc_as` contracts
do **not** deliver the caller's foundational *Uniqueness / non-aliasing*
guarantee, nor the *Monotone-capacity / Exhausted* boundary, nor
*no-spurious-consumption*; and the three proof lemmas that are meant to encode
those guarantees are **floating / orphan** (not referenced by any exec
contract). Detail below.

---

## Task 1 — SPEC QUALITY

### `align_up` (lib.rs:126-138, spec `align_up_spec` lib.spec.rs:57-68)
- ensures is a complete, bidirectional `match` over `Some`/`None`, pinning the
  result to the concrete, `open` `align_up_spec`. Both arms specified — no
  one-sided error. ✅
- `align_up_spec` is declarative (`((value+alignment-1)/alignment)*alignment`,
  `None` on `alignment==0` or `> usize::MAX`). Independent of impl strategy;
  passes the substitution test. ✅
- Minor: the caller-facing corollaries the caller-analysis lists (`r >= value`,
  `alignment | r`, `r < value + alignment`, already-aligned ⇒ `r == value`) are
  not surfaced as helper lemmas, but `align_up_spec` is `open` so callers can
  unfold and derive them at const-eval. Acceptable. (lib.spec.rs:57)

**Verdict:** correct, complete, understandable. ✅

### `BssStorage::as_mut_ptr` (lib.rs:200-204)
- ensures `result as int == base_of::<Self>()` — pins the returned pointer to
  the opaque ghost constant `base_of`, giving **stability** (same address every
  call). ✅
- Does **not** state base alignment (`base_of % A == 0`) or the `STORAGE_SIZE`
  writable-region size. Per `view_design.md` §4.2 and the `unsafe trait`
  contract, these are the implementor's `unsafe` duties (external-bottom), and
  `bump_view(self).inv()` *assumes* `base % unit_align == 0` (lib.spec.rs:125)
  rather than deriving it from `as_mut_ptr`. This is an acceptable
  trust-boundary split for an `unsafe` trait, but note nothing connects
  `bump_view(self).base` to `base_of::<S>()` — the binding promised in
  `view_design.md` §4.2 ("view().base is pinned to base_of") is **not present**
  in code. Minor; the precondition route makes it sound, but it weakens the
  `as_mut_ptr` ↔ View link. ⚠ (minor)

**Verdict:** minimal but acceptable (stability only). ⚠ minor gap noted.

### `alloc` (lib.rs:271-285)
```
requires bump_view(self).inv(),
ensures match result {
    Ok(slot) => { a = slot_ref_addr(slot);
        a % unit_align == 0 && base <= a && a + N <= base + storage_size }
    Err(_) => true,
}
```
- Success arm gives **per-slot Alignment + In-bounds**. ✅ (these two caller
  invariants are genuinely delivered.)
- **`Err(_) => true` is tautological** and drops *all* error meaning. `alloc`
  can return `Exhausted`, `Overflow`, `OutOfBounds`, `Misaligned`. The
  caller-analysis (caller_analysis.md:77-80) and the unit test
  `alloc_returns_exhausted_error` (lib.rs:483) rely on `Exhausted` marking the
  `NUM_UNITS` boundary, and on "no slot consumed on error". **None of this is in
  the contract.** Per the task, generic alloc-failure arms may legitimately be
  `true`, but here the *meaningful* `Exhausted` arm (the documented
  Monotone-capacity boundary) is also collapsed to `true`. ✗
- Success arm does **not** state `allocated + 1`, does **not** tie
  `slot_ref_addr(slot)` to `slot_addr(v.allocated)`, and gives **no uniqueness**
  relative to prior allocations. `view_design.md` §5.1 explicitly designed
  `slot as int == v.slot_addr(v.allocated)` and the `forall j` distinctness into
  the ensures; the implemented contract dropped both. ✗

**Bug-rejection test (spec-design principle #3):** a broken allocator that
returns the **same** aligned in-bounds slot on every call satisfies this `alloc`
contract. The spec is therefore *insufficient to reject* the most important bug
class (aliasing). ✗

### `alloc_as` (lib.rs:348-365)
- `Err(BumpAllocError::SizeMismatch) => size_of::<T>() != N` and
  `Err(BumpAllocError::AlignmentMismatch) => align_of::<T>() > A` — the
  **Stable-size** contract is specified on both the success arm (`size_of==N &&
  align_of<=A`) and these two error arms. ✅ (This is the well-specified part.)
- Same per-slot Alignment + In-bounds on `Ok`. ✅
- Same omissions as `alloc`: no uniqueness, no `allocated+1`, `Err(_) => true`
  for the propagated `alloc` errors (incl. `Exhausted`). ✗

### View (`BumpView`) design (lib.spec.rs:79-156)
- Fields are mathematical (`int`/`nat`), abstract, no atomics/pointers/
  `MaybeUninit`. Passes the per-field substitution test (view_design.md §6). ✅
- `inv()` (lib.spec.rs:116-133) is non-trivial: geometry well-formedness,
  `stride == align_up_spec(unit_size, unit_align)`, alignment divisibility,
  pool-fits-in-region, no-wrap, monotone ceiling. ✅
- `geometry_ok`, `spec_alloc`, `is_consumed`, `has_capacity`, `slot_addr` are
  well-designed helpers. ✅
- **But the View is `uninterp bump_view(...)` and is never connected to the
  returned references.** `slot_ref_addr(slot)` (lib.spec.rs:50, uninterp) is
  unconstrained beyond the per-call in-bounds+alignment, so none of the
  pool-level guarantees encoded in `inv()`/`geometry_ok`/`spec_alloc` reach the
  caller's actual `slot`. The View is a high-quality *model* that is **not bound
  to exec behavior** for uniqueness/transition. ✗ (see Task 2)

**Task 1 verdict:** `align_up` excellent; `alloc_as` size/align arms good;
`as_mut_ptr` minimal-but-ok; **`alloc`/`alloc_as` success+error arms are
materially incomplete** (uniqueness, Exhausted boundary, no-spurious-consumption
absent). **NOT fully satisfactory.**

---

## Task 2 — CALLER COVERAGE

Source: `caller_analysis.md` "Key Invariants" (lines 115-129) + per-function
expectations.

| # | Caller expectation | Where it should live | Delivered? |
|---|--------------------|----------------------|-----------|
| 1 | **Uniqueness / non-aliasing** (distinct slots; foundational unsafe soundness) | `alloc`/`alloc_as` Ok ensures, bound to returned ref | **❌ NO** — `slot_ref_addr(slot)` not tied to `slot_addr(i)`; two `alloc` results not provably distinct; `lemma_geometry` proves `slot_addr` injectivity but is **orphan** (never referenced by any exec contract) |
| 2 | **In-bounds** (slot ⊆ `[base, base+storage_size)`) | `alloc`/`alloc_as` Ok ensures | **✅ YES** — `base <= a && a + N <= base + storage_size` (lib.rs:280-281, 359-360) |
| 3 | **Alignment** (`a % A == 0`; `alloc_as`: `align_of<=A`) | `alloc`/`alloc_as` Ok ensures | **✅ YES** — `a % unit_align == 0` + `align_of::<T>() <= A` (lib.rs:279, 357-358) |
| 4 | **Monotone capacity / Exhausted boundary** (`(NUM_UNITS+1)`-th ⇒ `Exhausted`) | `alloc` Err(`Exhausted`) arm | **❌ NO** — `Err(_) => true`; `lemma_exhausted_boundary` is **orphan** |
| 5 | **Stable size contract** (`alloc_as` succeeds iff `size==N && align<=A`) | `alloc_as` Ok + SizeMismatch/AlignmentMismatch arms | **✅ YES** — lib.rs:356-363 (one-directional but covers the guard meaning) |
| 6 | **No spurious consumption on error** (faults don't burn/return a slot) | `alloc`/`alloc_as` Err arms | **❌ NO** — `Err(_) => true`; no `allocated` framing possible because View is `uninterp`/external_body |

**Covered: 3 / 6** (In-bounds, Alignment, Stable-size).
**Missing: 3 / 6** (Uniqueness, Monotone-capacity/Exhausted, No-spurious-consumption).

### On the "lemmas cover it" claim
The proof lemmas `lemma_geometry`, `lemma_alloc_transition`,
`lemma_exhausted_boundary` (lib.proof.rs:49,107,121) are mathematically correct
over `BumpView`, **but they are floating**: grep confirms **none** of
`geometry_ok | spec_alloc | has_capacity | is_consumed | slot_addr | lemma_*`
appears in any exec `requires`/`ensures` in `lib.rs`. Per spec-design ("No
floating specs — every spec function and lemma must ultimately connect to an
exec contract. Orphan specs are dead code." and "Standalone spec functions that
no exec function references prove nothing"), these lemmas do not discharge the
caller obligations. A caller holding a returned `&'static mut [u8; N]` cannot
invoke them to conclude its slot is distinct from another slot, because
`slot_ref_addr` is never equated to `slot_addr(i)`.

This is the documented consequence of deferring the `v → v'` atomic-ghost-token
transition (view_design.md §7; lib.spec.rs:12-17,164-176). The deferral is
*transparent and honest*, but at a **final** review the foundational Uniqueness
guarantee remains **undelivered by the verified contract**.

**Task 2 verdict:** 3/6 covered. Uniqueness/non-aliasing (the kernel's unsafe
soundness anchor) and the Exhausted/Monotone boundary are **not** covered by the
verified contract. **NOT satisfactory.**

---

## Task 3 — PROOF COMPLETENESS

- `admit()` occurrences in the three in-scope files: **0**
  (`grep -rn admit src/libs/bump_allocator/src` → none).
- `external_body` in the three in-scope files: **2**
  - `lib.rs:271` `#[verus_verify(external_body)]` on `alloc` (fn at lib.rs:286)
  - `lib.rs:348` `#[verus_verify(external_body)]` on `alloc_as` (fn at lib.rs:367)
- Both `external_body` appear in `tcb-allowed.md` (lines 16-23). ✅

**No `admit()` BLOCKER. No un-listed `external_body` BLOCKER.** ✅

---

## Task 4 — TCB COMPLIANCE

`tcb-allowed.md` lines 16-23 list both:
- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc` ✅
- `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc_as` ✅

`cheating-detail.txt` confirms exactly these two:
```
- lib.rs:286 alloc: external_body
- lib.rs:367 alloc_as: external_body
```
Every `external_body` is in the fixed TCB list. No new trust boundary
introduced. **TCB COMPLIANT.** ✅

---

## Task 5 — AST CONSISTENCY

```
# vs base branch verus-ai-prove-bottom-up
$ ast_consistency.py --base-ref verus-ai-prove-bottom-up .../lib.rs count
✅ Consistent: 12 functions, 7 structs match.

# vs pre-verus baseline (last non-verus commit e79991a92 "update bump allocator")
$ ast_consistency.py --base-ref e79991a92 .../lib.rs summary
Consistent: ✅ YES (matched=12 mismatched=0 missing=0 extra=0)
```
All 12 exec functions (incl. `alloc`, `alloc_as`, `align_up`,
`Backend*::as_mut_ptr`) and all 7 structs **MATCH**. No `// VERUS REWRITE` /
`// VERUS DEVIATION` / `// VERUS BUG FIX` comments exist (grep empty) — no exec
rewrites to scrutinize. Exec code is byte-faithful to the pre-verification
source.

**Task 5 verdict: PASS.** ✅ (no mismatch)

---

## Task 6 — VERIFICATION

```
$ make verify-bump-allocator   (forced, non-cached)
verification results:: 12 verified, 0 errors
Exit code : 0
cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0
coverage: 3/6 exec functions have contracts   (fmt/new/default uncontracted — out of scope)
status: CHEATING_DETECTED   ← solely the 2 TCB-listed external_body
```
- Verus: **12 verified, 0 errors, exit 0.** PASS.
- `status: CHEATING_DETECTED` is the harness flagging the 2 `external_body`,
  both pre-approved in `tcb-allowed.md` → allowed, not a blocker.
- `make build`: `make: Nothing to be done for 'build'` (no default build
  recipe; real build is `./z`). Crate-level `cargo build` + `cargo test`
  succeed: **3 unit tests + 1 doctest pass, exit 0.** Full `./z` kernel build
  not run (out of the quick-check budget) — noted as not-run, not blocking.

**Task 6 verdict: verification PASS (0 errors).**

---

## Task 7 — GUARDRAILS COMPLIANCE (exact counts)

Across the three in-scope files:

| Dimension | Count | Locations |
|-----------|-------|-----------|
| `admit` | **0** | — |
| `assume(` | **0** | — |
| `external_body` | **2** | lib.rs:271 (`alloc`), lib.rs:348 (`alloc_as`) — both in TCB ✅ |
| `assume_specification` | **1** | lib.spec.rs:28 (`<usize>::div_ceil`) |
| cfg-gated **exec** | **0** | only allowed forms present (below) |
| `trusted` / `no_decreases` / `spinoff` / `rlimit` | **0** | — |
| `uninterp spec fn` | **3** | lib.spec.rs:41 `base_of`, :50 `slot_ref_addr`, :177 `bump_view` |

- `admit == 0` and `assume == 0`: **no BLOCKER.** ✅
- `external_body` both in TCB: **no BLOCKER.** ✅
- **cfg gating audit:** the only `cfg` in `lib.rs` are `#[cfg(verus_keep_ghost)]`
  on the two `include!` lines (101, 105), `#[cfg(test)]` on the test module
  (392), and the crate `#![cfg_attr(not(any(test, feature="std")), no_std)]`
  (83). All are the explicitly-ALLOWED non-exec forms. **No exec branch / match
  arm / expression is cfg-gated.** ✅
- **`assume_specification [<usize>::div_ceil]` assessment:** searched the
  toolchain vstd (`/mnt/toolchain/verus/vstd`) — **no `div_ceil` spec exists**
  (`std_specs/num.rs` provides `checked_mul` etc. but not `div_ceil`). It is a
  std-library function vstd does not cover ⇒ a legitimate **external-bottom**
  boundary. Spec fidelity vs the std doc: `requires y != 0` (matches the
  documented zero-divisor panic), `ensures result == (x + y - 1) / y` (matches
  unsigned ceil-division, no overflow). **Acceptable; should NOT be removed.** ✅
- **`uninterp spec fn` note (×3):** spec-design bans `uninterp` *except* as a
  mechanical consequence of an `external_body` type/opaque boundary. Here:
  `base_of` = address of a static (opaque to Verus); `slot_ref_addr` = address
  of a `&mut` reference (not spec-readable — only raw pointers expose `.addr()`);
  `bump_view` = view over the interior-mutable `AtomicUsize` cursor (vstd has no
  support for reading atomic values) of the `external_body` `alloc`/`alloc_as`.
  All three are defensible mechanical consequences (same precedent accepted for
  `manager.rs::spec_kernel_watermark` in tcb-allowed.md:117-127). **Not a
  guardrail blocker**, but they are *why* the uniqueness guarantee cannot be
  bound (Task 2) — flagged for transparency.

**Task 7 verdict: no guardrail BLOCKER.** All cheating dimensions clean.

---

## Task 8 — SPEC DRIFT

```
$ spec_drift.py git-diff .../lib.rs --before HEAD
Functions with changes: 0
Contract drift (⚠ review required): 0
✅ No contract drift detected.   (exit 0)
```
Working tree matches committed verus state; no `requires` strengthened, no
`ensures` removed/weakened relative to HEAD. AST consistency additionally
confirms exec unchanged vs the pre-verus baseline. **No original guarantee was
weakened by an edit.** ✅

(Caveat: drift detection only catches *post-baseline edits*. The Uniqueness/
Exhausted gaps in Task 1/2 are *original under-specification*, not drift — they
were never present to be removed.)

**Task 8 verdict: PASS (no drift).**

---

## Task 9 — BUG RECONCILIATION

`bugs.md` states **"No code bugs found."** Reconciliation against final code:
- `align_up` (lib.rs:133-138): guards `alignment == 0`, uses
  `div_ceil(...).checked_mul(...)` ⇒ total, no panic/overflow. Matches
  `align_up_spec`. Claim holds. ✅
- `alloc` (lib.rs:286-322): every address step uses `checked_add`/`checked_mul`
  with `Overflow` fallback; validates `end > storage_end` (`OutOfBounds`) and
  `!ptr.is_multiple_of(A)` (`Misaligned`) before materializing the slot. No
  reachable overflow/OOB/misalignment on the success path. Claim holds. ✅
- `alloc_as` (lib.rs:367-378): checks `size_of::<T>()`/`align_of::<T>()` before
  touching storage. Claim holds. ✅
- CAS loop (lib.rs:288-301) reserves an index lock-free with an `Exhausted`
  guard and `checked_add` for the `next` counter. No overflow. ✅

No code bug was discovered during this review. **"No code bugs found" is
consistent with the final code state.** ✅

Note (not a code bug): the *spec* incompleteness in Task 1/2 is a
specification-coverage shortfall, classified under bug-reporting as **neither**
a True Bug nor Context-Dependent code bug — it is a spec gap. Correctly *not*
recorded in `bugs.md`. No unresolved *verification failure* exists (0 errors),
so there is nothing to classify under the bug-reporting skill.

---

## Issues ordered by severity

1. **[HIGH — spec quality] Uniqueness / non-aliasing not delivered.**
   `alloc`/`alloc_as` Ok arms (lib.rs:275-282, 352-361) bound only per-slot
   alignment+in-bounds over the uninterpreted `slot_ref_addr(slot)`; the return
   is never tied to `slot_addr(v.allocated)`, so two results are not provably
   distinct. A same-slot-every-call allocator satisfies the contract
   (spec-design #3 violation). `lemma_geometry` (lib.proof.rs:49) proves pool
   injectivity but is **orphan** (unreferenced by any exec contract).
2. **[HIGH — spec quality] Monotone-capacity / Exhausted boundary not in
   contract.** `Err(_) => true` (lib.rs:283, 364) drops the documented
   `Exhausted`-at-`NUM_UNITS` guarantee that `alloc_returns_exhausted_error`
   (lib.rs:483) and the kernel rely on. `lemma_exhausted_boundary`
   (lib.proof.rs:107) is orphan.
3. **[HIGH — spec quality] No-spurious-consumption on error not expressible.**
   With `bump_view` uninterp + `alloc` external_body, no `allocated`-framing on
   error arms exists; `Err(_) => true` is the only statement.
4. **[MEDIUM — spec design] Orphan lemmas.** `lemma_geometry`,
   `lemma_alloc_transition`, `lemma_exhausted_boundary` connect to no exec
   contract (grep-confirmed) — spec-design "no floating specs / prove nothing".
5. **[LOW] `as_mut_ptr` ↔ View link missing.** Code never binds
   `bump_view(self).base == base_of::<S>()` (view_design §4.2 promised it);
   soundness is routed through the `inv()` precondition instead. Minor.
6. **[INFO] 3× `uninterp spec fn`** — defensible mechanical consequences of
   external_body/opaque-address/atomic, but they are the mechanism by which
   issues 1-3 become unprovable at this phase.

All issues 1-6 are **specification-completeness** findings. There are **zero**
mechanical/guardrail blockers (admit, assume, un-listed external_body, AST
mismatch, drift, verification error are all clean).

---

## Final verdict

- Verification: **12 verified, 0 errors** (PASS).
- Cheating counts: `admit=0 assume=0 external_body=2(both TCB) assume_specification=1(div_ceil, std/ok) cfg_gate=0 trusted=0 no_decreases=0`; `uninterp=3` (defensible).
- AST: **PASS** (consistent vs base branch and vs pre-verus baseline).
- Spec drift: **PASS** (0).
- TCB: **COMPLIANT**.
- Blockers (mechanical, per task definition): **NONE**.
- Quality items satisfactory: **NO** — Uniqueness/non-aliasing,
  Monotone-capacity/Exhausted, and No-spurious-consumption (3 of 6 core caller
  invariants) are not delivered by the verified contract; supporting lemmas are
  floating/orphan.

Per the stated PASS rule ("PASS only if zero blockers **AND** every quality item
is satisfactory"), the unmet caller-coverage / spec-quality items prevent a
clean pass.

RESULT: FAIL
