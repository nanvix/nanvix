# Final Verification Review — `bump_allocator` (Verus)

> Independent, strict, read-only final review.
> Reviewer: GitHub Copilot CLI (Claude).
> Date: 2026-06-15.
> Scope (in-scope functions only): `FixedSizeBumpAllocator::alloc_as`,
> `FixedSizeBumpAllocator::alloc`, `align_up`, `BssStorage::as_mut_ptr`.
> Files reviewed: `src/libs/bump_allocator/src/{lib.rs, lib.spec.rs, lib.proof.rs}`,
> `caller_analysis.md`, `view_design.md`, `bugs.md`, `tcb-allowed.md`.
> Verification re-run independently: `make verify-bump-allocator` → **10 verified,
> 0 errors, exit 0** (fresh, non-cached run).

---

## Result line (read first)

- **No hard-rule guardrail blockers** (admit=0, assume=0, both `external_body` in
  `tcb-allowed.md`, no behavioral AST divergence).
- **But two review dimensions are NOT clean** — *Spec quality* and *Caller
  coverage* — because the central allocator safety contracts are absent or stated
  over disconnected opaque symbols, and one error-path postcondition is
  *over-strong relative to the real code* yet trusted via `external_body`.
- Per the review bar ("PASS only if zero blockers **and every dimension clean**"),
  the verdict is **FAIL**. Details below; the most serious findings are I-1 and I-2.

---

## Guardrails count line

```
admit=0  assume=0  external_body=2  assume_specification=0  cfg-gated-exec=0
```

Independently grepped (`src/libs/bump_allocator/src/`):

| Pattern | Real code occurrences | Locations |
|---|---|---|
| `admit()` | **0** | only a comment at `lib.proof.rs:7` |
| `assume(` | **0** | only a comment at `lib.proof.rs:7` |
| `external_body` | **2** | `lib.rs:303` (`alloc`), `lib.rs:380` (`alloc_as`) |
| `assume_specification` | **0** | only a comment at `lib.rs:142` |
| cfg-gated **exec** (`#[cfg(not(verus_keep_ghost))]` on branches/exprs/arms) | **0** | none |
| `uninterp spec fn` | 3 | `lib.spec.rs:27 base_of`, `:36 slot_ref_addr`, `:163 bump_view` |
| `// VERUS REWRITE` | 1 | `lib.rs:137` (`align_up`) |

The tool's own cheating check agrees: `assume=0 external_body=2 admit=0 trusted=0
no_decreases=0 cfg_gate=0`. `admit=0` and `assume=0` ⇒ **no hard guardrail
blocker** on these axes.

---

## 1. Spec quality

**align_up — GOOD.** `align_up_spec` (`lib.spec.rs:43`) is a concrete, total,
mathematically-typed (`nat`) function: least multiple of `alignment` that is
`>= value`, `None` iff `alignment == 0` or the product exceeds `usize::MAX`. The
`#[verus_spec]` on `align_up` (`lib.rs:126`) ties `Some`/`None` exactly to the
spec. This is a clean, caller-usable, non-tautological contract and matches every
caller expectation in `caller_analysis.md §align_up`.

**as_mut_ptr — adequate as a stability anchor.** `ensures result as int ==
base_of::<Self>()` (`lib.rs:232`) encodes "same address each call" (a fixed ghost
constant). The "≥ STORAGE_SIZE writable / exclusively-owned" portion is, by design
(`view_design.md §4.2`), the unsafe `BssStorage` TCB duty and is intentionally not
re-derived. Acceptable.

**BumpView design — internally well-formed.** The field set
(`base/stride/unit_size/unit_align/capacity/storage_size/allocated`) passes the
substitution test (`view_design.md §6`): every field is a property of the
*pool*, not of the `AtomicUsize` cursor. `inv()` (`lib.spec.rs:102`) encodes real
geometric constraints (non-empty slots, stride = `align_up`, A-aligned base,
pool-fits-region, no wrap, monotone ceiling), each justified clause-by-clause.
No implementation field (`next_slot`, `Ordering`, `MaybeUninit`) leaks in. As an
abstract model this is good work.

**alloc / alloc_as — the contracts are the problem (see Issues I-1, I-2, I-3).**
The two `external_body` functions carry the *only* caller-facing contracts, and
they fall short of both `caller_analysis.md` and the team's own `view_design.md
§5`:

- The entire `alloc`/`alloc_as` contract surface is phrased over **uninterpreted**
  symbols `bump_view(self)` and `slot_ref_addr(slot)` (both `uninterp`, with **no
  axioms** relating them to each other, to the concrete `N/A/S` constants, or to
  the real pointer the caller dereferences). `verus-constraints` explicitly warns
  that `uninterp spec fn` paired with `external_body` "has the same effect as
  `assume`." A caller cannot connect `slot_ref_addr(slot)` to `slot.as_ptr()`,
  so the geometry facts (`a % unit_align == 0`, in-bounds) are stated about an
  *opaque integer disconnected from anything the caller can act on*. Per
  spec-design this is "specs not written for the caller."
- The precondition `requires bump_view(self).inv()` (`lib.rs:305`, `:382`) is
  **not caller-dischargeable**: there is no broadcast/lemma/axiom anywhere that
  establishes `bump_view(self).inv()` (grep confirms — `inv()` appears only inside
  proof `requires`/`ensures`, never as an established fact). So in verified caller
  code the ensures are unreachable, and in unverified caller code they convey
  nothing.
- Missing clauses that `view_design.md §5` itself specified: the success-path
  **uniqueness / non-aliasing** `forall j ... slot != slot_addr(j)`, the
  consumption **transition** (`allocated+1`, `slot == slot_addr(allocated)`), and
  the **no-spurious-consumption-on-error** (`v'.allocated == v.allocated`).

**Verdict for dimension 1: NOT CLEAN.**

---

## 2. Caller coverage

Mapping every expectation in `caller_analysis.md` (per-function + the 6 "Key
Invariants") against an actual `requires`/`ensures`:

| Caller expectation (source) | Spec present? | Where / why |
|---|---|---|
| `align_up`: least-multiple / `None` iff `a==0`‖overflow | ✅ Covered | `align_up_spec` exact, `lib.rs:126` |
| `as_mut_ptr`: stable base address | ✅ Covered | `result == base_of::<Self>()`, `lib.rs:232` |
| `alloc`/`alloc_as`: in-bounds (`base ≤ a`, `a+N ≤ base+storage`) | ✅ Covered* | `lib.rs:311-313`, `:390-392` (*over opaque `slot_ref_addr`) |
| `alloc`/`alloc_as`: alignment (`a % unit_align == 0`) | ✅ Covered* | `lib.rs:311`, `:390` (*over opaque symbol) |
| `alloc_as`: type-match gating (`size_of==N`, `align_of≤A`) | ✅ Covered | `lib.rs:388-389`, `:395-401` |
| `alloc`/`alloc_as`: graceful exhaustion (Err, never panic) | ⚠️ Partial / over-strong | `Err ⇒ Exhausted` (`lib.rs:315`) — see I-2 |
| **Uniqueness / non-aliasing** (Key Invariant #1) | ❌ **MISSING** | no `forall j` distinctness in any ensures |
| Monotone consumption / `allocated+1` transition | ❌ MISSING | no `v→v'` clause (deferred, `view_design §5` note) |
| No spurious consumption on error | ❌ MISSING | no `v'.allocated == v.allocated` clause |
| Thread-safe handout (no index twice) (Key Inv. #5) | ❌ MISSING | no concurrency/atomic model (deferred) |
| Stability `'static` (Key Inv. #6) | ⚠️ Type-level only | `&'static mut` in signature; no spec clause |

**Key-Invariant coverage: 3 / 6 fully covered** (In-bounds+well-formed,
Type-match gating, and the Exhaustion *path* of graceful-exhaustion). **Missing:
Uniqueness/non-aliasing (#1), Thread-safe handout (#5), and the spec-level
monotone-consumption** that backs "at most NUM_UNITS handed out." Stability (#6)
is type-level only.

The single most important property for an allocator that hands out `&'static mut`
— **non-aliasing** — is absent from the verified surface. `caller_analysis.md`
states verbatim "Would break callers if: a returned reference aliased an earlier
slot." That property is not proven and not even asserted.

**Caller Coverage: 3/6 Key Invariants. Verdict: NOT CLEAN.**

---

## 3. Proof completeness

- `admit()` count: **0**. `assume(...)` count: **0**. (No hard blocker.)
- `external_body` count: **2** — `alloc` (`lib.rs:318`, attr `:303`) and `alloc_as`
  (`lib.rs:405`, attr `:380`). **Both are explicitly listed in `tcb-allowed.md`**
  (lines 10–17). ✅ No unapproved `external_body`.
- The discharged proofs are real: `lemma_ceil_div` (`lib.proof.rs:22`) is fully
  proven **and actually invoked** by `align_up` exec (`lib.rs:167`) — a genuine
  proof→exec connection.
- **However, three lemmas are proven but never wired to any exec contract:**
  `lemma_geometry` (`:64`), `lemma_exhausted_boundary` (`:126`), and
  `lemma_alloc_transition` (`:140`) have **zero call sites** in exec code
  (grep-confirmed). Because `alloc`/`alloc_as` are `external_body`, they cannot and
  do not use these lemmas. The lemmas therefore prove facts about an abstract
  `BumpView` that the real allocator is **never shown to instantiate** — they raise
  no assurance about the shipping code (Issue I-3). The "6/10 verified" count is
  honest arithmetic, but most of the proof effort is decoupled from the verified
  exec surface.

**Verdict: no `admit`/`assume`; `external_body` approved; but proof relevance to
exec is weak (I-3).**

---

## 4. TCB compliance

Both `external_body` sites are pre-approved:

- `tcb-allowed.md:10-14` → `FixedSizeBumpAllocator::alloc`.
- `tcb-allowed.md:15-17` → `FixedSizeBumpAllocator::alloc_as`.

No `external_body` exists outside this list. No new trust boundary is introduced
in scope. ✅ **TCB-compliant (no blocker).**

Note (not a TCB-list violation, but flagged for honesty): `verus-constraints`
classifies `uninterp spec fn` as banned and `external_body` on a module's own
functions as forbidden. This effort relies on both, justified by the
project-specific `tcb-allowed.md` and the `raw-array` precedent. The `external_body`
is genuinely covered by the allow-list. The `uninterp` symbols are **not** governed
by any allow-list and are the root of I-1/I-2 (their opacity is what makes the
contracts inert and lets the over-strong error ensures pass unchecked).

---

## 5. AST consistency

Tool: `scripts/ast_consistency.py --base-ref c6f37acae^` (the pre-Verus commit
`e79991a92`).

```
Consistent: ❌ NO (matched=11 mismatched=1 missing=0 extra=0)
align_up  →  MISMATCH   (all other 11 functions + 7 structs MATCH)
```

The sole exec change is the documented `// VERUS REWRITE` in `align_up`:

```
- value.div_ceil(alignment).checked_mul(alignment)
+ let r = value % alignment; let qd = value / alignment;
+ let q = if r == 0 { qd } else { qd + 1 };
+ q.checked_mul(alignment)
```

**Semantic-equivalence audit (rigorous):**

1. For `alignment > 0` (guaranteed by the early `if alignment == 0 { return None }`),
   Rust's `usize::div_ceil` is *defined* as `let d=self/rhs; let r=self%rhs;
   if r>0 { d+1 } else { d }`. The rewrite computes exactly that `q`. **Identical
   value for all inputs.** The trailing `.checked_mul(alignment)` is unchanged.
2. The rewrite ties `q` to the spec ceiling form `(value+alignment-1)/alignment`
   via `lemma_ceil_div` (`lib.proof.rs:22`, proven by `lemma_fundamental_div_mod`
   + `nonlinear_arith`). Correct.
3. **Overflow argument** (the one place open-coding could differ): `qd + 1` is an
   *unchecked* exec add. It cannot overflow: `r != 0` forces `alignment >= 2`
   (since `alignment == 1` ⇒ `r == 0`), hence `qd = value/alignment <= value/2 <
   usize::MAX`, so `qd + 1 <= usize::MAX`. Verus proves this inline
   (`lib.rs:152-164`, `assert(qd < usize::MAX)`). `div_ceil` performs the same
   `d+1` internally, so no behavioral divergence is introduced.
4. The original `div_ceil` truly has **no vstd spec** — independently confirmed:
   `grep -rn div_ceil <verus>/vstd` returns nothing; the minimal reproducer
   `cheating-elimination/repro/div_ceil_no_spec.rs` exists and captures the real
   Verus error ("`core::num::<impl usize>::div_ceil` is not supported").

The AST checker flags MISMATCH because the exec text changed, but the change is a
**semantically-equivalent, documented, reproducer-backed pre-approved deviation**
(no behavioral divergence for any input). There is **no behavioral AST mismatch**.

**AST consistency (behavioral): PASS.** (Textual MISMATCH is expected and
justified for the single rewrite; the reproducer reference is present.)

---

## 6. Verification

Independent fresh run (touched `lib.rs` to defeat the cache):

```
verification results:: 10 verified, 0 errors
Exit code : 0
cheating: assume=0 external_body=2 admit=0 trusted=0 no_decreases=0 cfg_gate=0
coverage: 3/6 exec functions have contracts
status: CHEATING_DETECTED   (solely because external_body=2 — both TCB-approved)
```

- Exit 0 / 0 errors: ✅ confirmed.
- Cheating counts match the pre-computed context exactly. The "CHEATING_DETECTED"
  status is driven only by the two allow-listed `external_body` and is therefore
  expected; it is **not** an admit/assume hit.
- Coverage 3/6 exec functions with contracts; the 3 without (`fmt`, `new`,
  `default`) are all **out of scope**. ✅

---

## 7. Guardrails compliance

`admit=0`, `assume=0` ⇒ **no hard blocker.** `external_body=2` (both approved),
`assume_specification=0`, cfg-gated exec=0. All counts independently reproduced.
See the Guardrails count line above. ✅ (on the guardrail axes specifically).

---

## 8. Bug reconciliation

`bugs.md` records **"No code bugs found"** in both the specification and proving
phases, and notes `alloc`/`alloc_as` were (re)set to `external_body`.

Reconciliation against the final code:

- The claim "`align_up` is total and correct … Verified with 0 errors" — **still
  valid.** Confirmed by re-run and equivalence audit (§5).
- The claim "`alloc` defensively uses `checked_add`/`checked_mul` … no reachable
  overflow/OOB/misalignment on the success path" — **true of the exec body**, but
  `bugs.md` does **not** record the consequence that the shipped `external_body`
  ensures `Err(e) => e == Exhausted` (`lib.rs:315`) **contradicts** that very body,
  which *can* return `Overflow`/`OutOfBounds`/`Misaligned` (`lib.rs:325/336/339/340/343/345/348`).
  Per `bug-reporting`, this surviving spec-vs-code discrepancy should have been
  classified — it is a **False-Positive-masking-by-trust**: a checked body would
  fail this ensures (the boundary `base+storage_size == usize::MAX+1` permitted by
  `inv()` clause (d) is a concrete counterexample → `Err(Overflow)`), but
  `external_body` suppresses the check. This is recorded here as **I-2**; it was
  **not** recorded in `bugs.md`.
- No *true code bug* in the exec logic was found by this review either — the
  allocator implementation itself is defensive and correct. The issues are
  **specification/coverage**, not exec bugs. So `bugs.md`'s "no code bug"
  conclusion is accurate **for exec code**, but it under-reports the
  spec-integrity gap (I-2) that the proving phase should have surfaced.

**Bug Summary:** 0 exec bugs (consistent with `bugs.md`); **1 unrecorded
spec-integrity issue** — an `external_body`-trusted error-path postcondition that
is stronger than the real code guarantees (I-2).

---

## Issues (highest priority first)

**I-1 — Uniqueness / non-aliasing is unverified and unasserted.** *(Spec quality,
Caller coverage)* `alloc` (`lib.rs:304-316`) and `alloc_as` (`lib.rs:381-403`)
success ensures contain only alignment + in-bounds. The #1 caller invariant
("no two live `&'static mut` alias", `caller_analysis.md:129`) — and the
`forall j … slot != slot_addr(j)` clause that `view_design.md §5.1` itself
specified — is **absent**. The supporting proof (`lemma_geometry`'s distinctness)
exists but is never connected to the exec contract. The core safety property of
the allocator is therefore not established.

**I-2 — Over-strong error postcondition trusted via `external_body`.** *(Spec
quality, Soundness)* `alloc` ensures `Err(e) => e == BumpAllocError::Exhausted`
(`lib.rs:315`), but the body can return `Overflow`/`OutOfBounds`/`Misaligned`
(`lib.rs:325,336,339,340,343,345,348`). `alloc_as` similarly restricts errors to
`{SizeMismatch, AlignmentMismatch, Exhausted}` (`lib.rs:395-397`) yet propagates
`alloc`'s other error variants via `self.alloc()?` (`lib.rs:413`). Even granting
`inv()`, clause (d) (`base + storage_size <= usize::MAX + 1`) permits the boundary
`== usize::MAX+1`, making `base.checked_add(STORAGE_SIZE)` (`lib.rs:341-343`)
return `Err(Overflow)` — a concrete counterexample to the ensures. Because the
functions are `external_body`, this postcondition is **trusted, not checked**; a
checked proof would fail. This is a latent unsoundness if any caller ever
discharges the precondition.

**I-3 — Contract surface and most proofs are stated over disconnected `uninterp`
symbols.** *(Spec quality, Proof relevance)* `bump_view`, `slot_ref_addr`, and
`base_of` are `uninterp` with **no axioms** linking them to the real allocator
state or to `slot.as_ptr()`. `verus-constraints` flags `uninterp + external_body`
as "the same effect as `assume`." Consequently: (a) the precondition `bump_view
(self).inv()` is not caller-dischargeable (no establishing lemma/broadcast
exists); (b) the geometry facts are about an opaque integer, not the caller's real
pointer; (c) `lemma_geometry`/`lemma_exhausted_boundary`/`lemma_alloc_transition`
are proven but uncalled, raising assurance about a model the shipping code is never
shown to satisfy. This is weaker than the `raw-array` precedent it cites (there the
single `view()` anchor is related *across* every operation; here two separate
opaque symbols are related only *within* a trusted body).

**I-4 — No-spurious-consumption and monotone-capacity transition are unspecified.**
*(Caller coverage)* No ensures states `v'.allocated == v.allocated` on error or
`v'.allocated == v.allocated + 1` on success. `view_design.md §5` listed both;
they are deferred to the (not-yet-done) atomic-ghost phase. Acknowledged as a
documented deferral, but it leaves "at most NUM_UNITS handed out" and "errors burn
no slot" unverified.

**I-5 — Thread-safe handout / `'static` stability unspecified at the spec level.**
*(Caller coverage)* Key Invariants #5 and #6 have no spec representation
(concurrency model deferred; `'static` is only the return type). Documented
deferral.

None of I-1…I-5 is a *hard-rule* blocker (no `admit`/`assume`, no unapproved
`external_body`, no behavioral AST mismatch). I-1, I-2, and I-3 are nonetheless
**dimension-level failures** that prevent a clean PASS.

---

## Summary scorecard

| Dimension | Verdict |
|---|---|
| 1. Spec quality | ❌ NOT CLEAN (I-1, I-2, I-3) |
| 2. Caller coverage | ❌ NOT CLEAN — **3/6 Key Invariants** (uniqueness, thread-safety, monotone-consumption missing) |
| 3. Proof completeness | ⚠️ admit/assume=0, external_body approved, but 3 lemmas decoupled from exec (I-3) |
| 4. TCB compliance | ✅ CLEAN (both `external_body` allow-listed) |
| 5. AST consistency | ✅ PASS (behavioral) — single justified, equivalent `align_up` rewrite |
| 6. Verification | ✅ 10 verified, 0 errors, exit 0 |
| 7. Guardrails | ✅ admit=0 assume=0 (no hard blocker); external_body=2 approved |
| 8. Bug reconciliation | ⚠️ 0 exec bugs, but I-2 spec-integrity issue unrecorded in `bugs.md` |

- **Guardrails:** `admit=0 assume=0 external_body=2 assume_specification=0
  cfg-gated-exec=0`
- **AST consistency:** PASS (behavioral; 1 justified textual rewrite in `align_up`)
- **Caller Coverage:** 3/6 Key Invariants
- **Bug Summary:** 0 exec bugs; 1 unrecorded over-strong-error-ensures spec issue (I-2)

---

## Result: FAIL

**Rationale.** There are **no hard-rule guardrail blockers** — `admit=0`,
`assume=0`, both `external_body` are in `tcb-allowed.md`, and the only AST change
(`align_up`) is a rigorously-verified semantically-equivalent rewrite with a real
Verus-error reproducer. `align_up` and the numeric/geometry lemmas are genuinely
and soundly proven, and `make verify-bump-allocator` is clean (10 verified, 0
errors).

However, the review bar is "PASS only if zero blockers **and every dimension
clean**," and **Spec quality** and **Caller coverage** are not clean:

1. The allocator's defining safety property — **uniqueness / non-aliasing** of
   handed-out `&'static mut` slots — is neither proven nor even asserted (I-1).
2. `alloc`/`alloc_as` assert an **error-path postcondition stronger than the real
   code** (`Err ⇒ Exhausted` / restricted error set), trusted only because the
   functions are `external_body` (I-2) — a latent unsoundness.
3. The entire `alloc`/`alloc_as` contract is phrased over **uninterpreted symbols
   with no establishing axioms**, making the precondition non-dischargeable and the
   geometry facts inert for callers, while the geometry/transition lemmas remain
   **decoupled from the verified exec surface** (I-3).

The deliverable verifies a self-consistent *abstract pool model* and a correct
`align_up`, but does **not** verify the safety contract the caller analysis says
the kernel depends on. A strict final review cannot pass it. Recommended path to
PASS: attach `BumpView` via an atomic-ghost / `PointsTo` token so `inv()` is
establishable and `bump_view`/`slot_ref_addr` become interpreted, then surface
the uniqueness, transition, no-spurious-consumption, and *accurate* error-set
clauses on `alloc`/`alloc_as` (ideally as checked — not `external_body`-trusted —
postconditions), wiring in the already-proven `lemma_geometry` /
`lemma_alloc_transition` / `lemma_exhausted_boundary`.
