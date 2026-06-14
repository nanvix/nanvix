# Final Verification Review — `mm::phys` module (phys-mod)

**Reviewer:** Claude (independent, strict)
**Date:** 2026-06-15
**Branch:** verus-ai-prove
**Scope (ONLY):** `init`, `book_physical_memory_regions`, `book_mmio_regions` in
`src/kernel/src/mm/phys/mod.rs`.
**Mode:** read-only. No files modified.

In-scope files:
- `src/kernel/src/mm/phys/mod.rs` (source + contracts)
- `src/kernel/src/mm/phys/mod.spec.rs` (spec functions / view)
- `src/kernel/src/mm/phys/mod.proof.rs` (proof helpers — empty)

Sibling modules (`frame`, `kframe`, `manager`, `upool`) are SEPARATE verification
targets; their `admit`/`external_body` are out of scope and reported only as
kernel-wide context.

---

## 1. Spec Quality

### 1.1 External-top API (`init`)

`init` is the only public / externally-observed function (sole caller
`kernel_vas::init`, `kernel_vas.rs:120`). Its `#[verus_spec]`:

- `requires phys_view().inv()`
- `ensures phys_view().inv()` (both arms) and, on `Ok`:
  - `phys_view().live()` (initialized ∧ manager_ready ∧ frames.wf())
  - `frames.all_reserved(phys_regions_frame_set(&physical_memory_regions))`
  - `forall a: mmio_regions_frame_set(mmio_regions).contains(a) ∧ frames.covers(a) ==> frames.reserved(a)`

Assessment against spec-design criteria:
- **Declarative / written-for-caller:** ✅ The three Ok facts are exactly the
  caller-relied-upon properties (subsystem live; physical regions reserved;
  covered MMIO frames reserved). They are phrased over abstract frame sets
  (`Set<int>`), not over the bitmap/`AtomicBool`/refcount-slice mechanism, and
  pass the substitution test (a buddy/free-list rewrite would still satisfy them).
- **Sufficient to reject bugs:** ✅ for the safety property. A buggy `init` that
  returns `Ok` while leaving a singleton uninitialized is rejected by
  `live()`; one that fails to book a physical region is rejected by
  `all_reserved(...)`; one that returns a booked MMIO frame from a later
  `alloc()` is rejected by the covered⟹reserved clause.
- **Not tautological / not subsumed:** ✅ `live()` is not implied by `inv()`
  (inv only gives `initialized ==> wf`); `all_reserved` and the MMIO clause add
  genuine content.

Advisory weaknesses (NOT blockers — see §2 for caller-justification):
- The final `init` Ok-arm does **not** pin the exact post-state
  (`v'.frames == seed(..).book_all(P).book_covered(M)` from view_design §4.4 was
  dropped). Consequence: an implementation that *over-reserves* (reserves frames
  beyond `P ∪ covered(M)`) would still satisfy the spec. This is conservative —
  over-reserving never violates the caller's "booked ⇒ never allocated" safety
  property — but it is a small loss of completeness.
- The "uncovered MMIO frame left unchanged" half (encoded by `book_covered` in
  view_design §4.3) is not stated; only the positive covered⟹reserved half is.
  The caller is warned to NOT assume uncovered frames are booked, so the negative
  half is not relied upon. Advisory only.

### 1.2 Private helpers (`book_*`)

Both are private, reachable only through `init` (confirmed by caller_analysis).
Their contracts exist solely to support `init`'s post-state, and both correctly:
- `requires phys_view().initialized ∧ phys_view().inv()`
- `ensures phys_view().inv() ∧ phys_view().initialized` (both arms), plus the
  per-helper Ok reserved-set facts.

These are appropriately scoped helper contracts.

### 1.3 `Err(_) => true` — FIRM VERDICT: **NOT a one-sided / tautological-error violation.**

The match arms are literally `Err(_) => true`, but each function carries an
**unconditional** ensures *outside* the match:
- `init`: unconditional `phys_view().inv()`.
- `book_physical_memory_regions` / `book_mmio_regions`: unconditional
  `phys_view().inv()` **and** `phys_view().initialized`.

Therefore the *effective* error postconditions are:
- helpers: `initialized ∧ inv()` ⟹ (via `inv`) `frames.wf()` — i.e. the frame
  partition remains well-formed after a partial booking failure. This is a real,
  non-trivial constraint and matches the view_design §4.2/4.3 promised
  `v'.frames.wf()` Err arm.
- `init`: `inv()` only. Since `init` does not also promise `initialized` on Err,
  this arm can be vacuously satisfied when `!initialized`. It is the weakest arm,
  but `inv()` is still a genuine constraint, and it exactly matches the *only*
  thing the caller observes on Err (boot aborts via `?`; no partial state is
  inspected — caller_analysis L91–96, view_design §4.4).

This is a caller-justified, non-transactional error path — not the anti-pattern
#5 "One-Sided Error Spec" (which is error arm == literally nothing). The error
arm here carries `inv()`/`wf` preservation. **Acceptable.**

Advisory: `init`'s Err arm could be marginally strengthened (e.g. assert
`frames.wf()` whenever `frame::init` already succeeded) and
`book_physical_memory_regions` dropped the view_design `!all_free(R)` fail-fast
conflict predicate — but neither is required by any caller.

### 1.4 `uninterp spec fn` (4 occurrences) — FIRM VERDICT: **all four QUALIFY as mechanical consequences of external boundaries; none is a verification escape.**

verus-constraints bans `uninterp` EXCEPT as a mechanical consequence of an
external_body type / external-bottom boundary. Per-function verdict:

| uninterp fn | Loc | Verdict | Justification |
|---|---|---|---|
| `byte_at_address(int) -> u8` | mod.spec.rs:13 | ✅ Qualifies (pre-existing, protected) | In the do-not-modify set; an external-bottom raw-memory byte accessor (machine memory is outside Verus's model). Out of this review's modification scope. |
| `phys_view() -> PhysModView` | mod.spec.rs:98 | ✅ Qualifies (singleton-global ghost) | The subsystem state lives in module-level `static mut` singletons (`frame::INSTANCE`/`INSTANCE_INIT`, manager/`Upool`), which have **no `self`** to attach `View::view()` to. It is the parameter-free global-state accessor whose value is pinned in the proving phase by the external_body `frame::instance` (in tcb-allowed.md) + a ghost token (view_design §8). Same established pattern as `identity_map_view()`. Morally equivalent to `View::view()` on an external_body singleton — a mechanical consequence of the external-bottom singleton boundary, not an escape. |
| `phys_regions_frame_set(&LinkedList<…>) -> Set<int>` | mod.spec.rs:177 | ✅ Qualifies (foreign external type) | A **direct** mechanical consequence of `LinkedList` being registered as an external type (`ExLinkedList`, `external_type_specification`/`external_body`) with no Verus `View`/iterator model. You cannot fold a concrete recursive definition over a foreign type vstd cannot see into. The function names the abstract union of `region_frame_addrs` over the list — the only thing the contract needs, and it is deterministic in the list. |
| `mmio_regions_frame_set(&LinkedList<…>) -> Set<int>` | mod.spec.rs:183 | ✅ Qualifies (foreign external type) | Same as above (MMIO list after GVA→GPA). |

Most contestable is `phys_view()`: it is not *literally* `View::view()` of an
external_body type (there is no struct in `mod.rs` to implement `View` for — the
state is global). However the carve-out's *intent* (the trust obligation is
tracked at the external boundary, here the external_body `frame::instance` in the
TCB list + the §8 ghost token) is satisfied, and this is an already-established
codebase convention (`identity_map_view`). I accept it. Were `phys_view()`
unbacked by any external boundary it would be a violation; it is backed.

**Net:** the `uninterp` usage is acceptable. It does not, in combination with the
two `external_body` helpers, smuggle in an arbitrary axiom: `phys_regions_frame_set`
/ `mmio_regions_frame_set` are deterministic functions of their inputs, so the
caller can chain `init`'s reserved-set facts about the *same* list.

---

## 2. Caller Coverage

Source of truth: `caller_analysis.md` "Caller Expectations" + "Key Invariants".
Sole caller: `kernel_vas::init`. Mapping each caller-relied-upon property to a
contract clause:

| # | Caller expectation | Contract clause | Status |
|---|---|---|---|
| 1 | One-shot init; allocator live & consistent after Ok | `init` Ok ⟹ `live()` (initialized ∧ manager_ready ∧ wf) | ✅ Covered (liveness). One-shot enforced **dynamically** (frame::init Errs on 2nd call); static `requires !initialized` dropped — see note A |
| 2 | Every physical-region frame booked (booked ⇒ never allocated) | `init` Ok ⟹ `frames.all_reserved(phys_regions_frame_set(..))` | ✅ Covered |
| 3 | Covered MMIO frames booked; above-RAM skipped | `init` Ok ⟹ `∀a ∈ mmio set: covers(a) ⟹ reserved(a)` | ✅ Covered (positive half). Negative "uncovered unchanged" half dropped — note B |
| 4 | Manager + fresh Upool live | `live()` includes `manager_ready` | ✅ Covered |
| 5 | Subsystem consistent (wf / inv) | `inv()` (both arms) + `wf` in `live()` | ✅ Covered |
| 6 | After Err: boot aborts, only error surfaced, no partial state relied on | `Err => true` + unconditional `inv()` | ✅ Covered (matches caller exactly) |
| 7 | `book_physical_memory_regions` Ok: all region frames booked | helper Ok ⟹ `all_reserved(phys_regions_frame_set(..))` | ✅ Covered |
| 8 | `book_physical_memory_regions` Err: propagate, partition stays wf | helper Err ⟹ `inv() ∧ initialized` ⟹ wf | ✅ Covered |
| 9 | `book_mmio_regions` Ok: covered booked, uncovered skipped | helper Ok ⟹ covered⟹reserved | ✅ Covered (positive half) |
| 10 | `book_mmio_regions` Err: propagate, wf preserved | helper Err ⟹ `inv() ∧ initialized` | ✅ Covered |

**Caller Coverage: 10/10 caller-relied-upon expectations covered**
(equivalently 5/5 "Key Invariants": one-shot, booked⇒never-allocated,
coverage-gated MMIO skip, wf-preserved, fail-fast).

**Missing (genuine caller guarantee lost): NONE.**

### Properties proposed in view_design.md that were DROPPED — judgment

- **Note A — `init requires !v.initialized` (one-shot, static):** DROPPED. Final
  `init` only `requires inv()`. Effect: `init` is callable when already
  initialized; a second call hits `frame::init`'s runtime rejection → `Err`
  (safe; caller aborts). Weaker `requires` = more permissive function, **not
  unsound**. One-shot is enforced *dynamically* instead of *statically*. The
  caller calls `init` exactly once at boot, so no real guarantee is lost.
  Advisory.
- **Note B — composed transition `v'.frames == seed(..).book_all(P).book_covered(M)`
  and the "uncovered unchanged" half:** DROPPED. view_design itself flagged the
  headline safety fact (3) as the form the caller actually uses, and the final
  spec keeps (3) directly. Dropping (2) loses exactness (over-reservation would
  pass) but loses no caller safety guarantee (over-reserving is conservative).
  Advisory.
- **Note C — `Err` arms `!all_free(R)` / explicit `wf`:** `wf` is preserved via
  `inv()`+`initialized`; the `!all_free(R)` conflict predicate (fail-fast
  richness) was dropped. The caller relies only on the error being surfaced
  (caller_analysis L105–107 about *consistency* of reporting is satisfied: a
  conflict always yields `Err`, never silent success — that is the body's `?`
  propagation, captured by the absence of an Ok guarantee on conflict). Advisory.

None of the dropped properties weakens a guarantee the sole caller actually
consumes. They are documented, deliberate, caller-justified reductions.

---

## 3. Proof Completeness (phys-mod files ONLY)

Grep of the three phys-mod files:

- `admit()` in phys-mod: **0** (strict scan `\badmit\b`: NONE). ✅
- `assume(...)` in phys-mod: **0**. ✅
- `external_body` in phys-mod: **2** functions — `mod.rs:73`
  `book_physical_memory_regions`, `mod.rs:103` `book_mmio_regions`. Both are
  pre-listed in `tcb-allowed.md` (lines 74–81). ✅
- `mod.proof.rs`: empty (no helpers needed; comment explains `init` discharges its
  postcondition directly from dependency contracts). ✅
- `init` itself is **body-verified** (`#[verus_spec]` only; NOT
  `#[verus_verify(external_body)]`). ✅

No `admit()` and no un-listed `external_body` exist in phys-mod ⟹ no BLOCKER from
this section.

---

## 4. TCB Compliance

- `book_physical_memory_regions` (mod.rs:73) — pre-listed (tcb-allowed.md L74). ✅
- `book_mmio_regions` (mod.rs:103) — pre-listed (tcb-allowed.md L79). ✅

Both external_body boundaries are justified by the documented LinkedList
limitation (no vstd model; orphan rule E0117 blocks providing `View` /
`ForLoopGhostIterator` from the kernel crate). Matches `bugs.md`.

**`ExLinkedList` external_type_specification (mod.spec.rs:65–69):** assessed.
This registers the foreign `alloc::collections::LinkedList` so it can appear in
spec signatures — the *prescribed* mechanism per the verus-constraints skill
("If the struct truly cannot be parsed by Verus, use `external_type_specification`
in spec.rs"). The cheating detector classifies it as `external_type_spec`, a
category **separate** from `external_body` (it is NOT included in the
external_body=18 count). Only the type is registered; no `View`/iterator spec is
provided (which is exactly why the two helpers must be external_body). Its
rationale is documented in mod.spec.rs L50–61, bugs.md, and the LinkedList
paragraphs of tcb-allowed.md (L74–81).

Advisory: `ExLinkedList` is not given its *own* dedicated line in tcb-allowed.md's
allowed-list the way `ExFrameNumber`/`ExPageTableBss` are; its justification lives
in the prose of the two `book_*` entries. This is a documentation-tidiness nit,
not a trust-boundary violation — `external_type_specification` of a genuinely
foreign std type is skill-sanctioned and is the only way LinkedList can be named
in specs. **Not a blocker.**

No new/unapproved trust boundary was introduced.

---

## 5. Guardrails Compliance — exact counts (PHYS-MOD ONLY)

Grep over `mod.rs`, `mod.spec.rs`, `mod.proof.rs`:

| Dimension | phys-mod count | Locations |
|---|---:|---|
| `admit()` | **0** | — |
| `assume(...)` | **0** | — |
| `external_body` (functions) | **2** | mod.rs:73, mod.rs:103 (both in tcb-allowed.md) |
| `external_type_specification` | **1** | mod.spec.rs:69 `ExLinkedList` (foreign std type; skill-sanctioned) |
| `assume_specification` | **0** | — |
| `uninterp spec fn` | **4** | mod.spec.rs:13, 98, 177, 183 (all qualify, §1.4) |
| cfg-gated **exec** code | **0** | only `#[cfg(feature="test")]` on the test module + `#[cfg(verus_keep_ghost)]` on spec/proof includes & the vstd import — all non-semantic, allowed |
| `// VERUS REWRITE` | **0** | — |
| `#[verifier::trusted]` / `#[verifier::external]` | **0** | — |

phys-mod: `admit=0, assume=0` ⟹ no guardrail blocker. The 2 external_body are in
the TCB list. `cfg(verus_keep_ghost)` gates only the `include!` of spec/proof and
the cfg-gated `use ::vstd::prelude::*` — no exec branch/match/expression is
cfg-gated.

### Kernel-wide totals (context only — span ALL in-progress sibling modules, NOT phys-mod)

From `make verify-kernel MODULE=mm::phys` global tally:
`assume=0  external_body=18  admit=27  trusted=0  no_decreases=0  cfg_gate=15`.

These include `frame`, `kframe`, `manager`, `upool`, `hal::mem::*`,
`mm::virt::identity_map`, `arch::*`, `bump_allocator` etc. — **out of scope** for
this review. phys-mod's own slice of that total is: external_body=2 (book_*),
external_type_spec=1 (ExLinkedList), admit=0, assume=0.

---

## 6. AST Consistency

```
$ python3 .../ast_consistency.py src/kernel/src/mm/phys/mod.rs count
✅ Consistent: 4 functions, 0 structs match.

$ ... summary
book_mmio_regions / book_physical_memory_regions / init / test → all MATCH
Consistent: ✅ YES (matched=4 mismatched=0 missing=0 extra=0)
```

- No `// VERUS REWRITE` comments anywhere in phys-mod (grep: 0). Nothing to
  semantically reconcile.
- spec-drift: "No contract drift detected" (0 functions changed) — exec
  signatures untouched (attribute-style specs only).

**No mismatch. ✅ Not a blocker.**

---

## 7. Verification

```
$ make verify-kernel MODULE=mm::phys
note: verifying module mm::phys
note: verifying module mm::phys::frame / kframe / manager / upool
Exit code : 0
verification: cached (no recompilation), — (exit 0)
```

Exit **0**, zero verification errors. (`status: CHEATING_DETECTED` in the summary
refers to the **kernel-wide** admit/external_body tallies from the still-in-progress
sibling modules — it is NOT a phys-mod verification failure; phys-mod contributes
admit=0.) **PASS.**

---

## 8. Bug Summary (reconciliation vs final code)

`bugs.md` contains:
1. **Code bugs: "None found."** — Reconciled against final `mod.rs`: the three
   target functions have no overflow / off-by-one / impossible path. The MMIO
   loop computes `end = start + (size-1)` and gates booking on
   `frame::is_covered`, matching the covered⟹reserved contract. Confirmed: still
   valid, no code bug.
2. **Verifier limitation — LinkedList iteration** (not a code bug). Reconciled:
   `book_physical_memory_regions`/`book_mmio_regions` iterate
   `alloc::collections::LinkedList` via `for region in list.iter()`; vstd has no
   model and the orphan rule blocks providing one. This **justifies the 2
   external_body** and the `ExLinkedList` registration. Matches tcb-allowed.md
   and the final code exactly. Classified (bug-reporting skill) as an
   **environment/verifier limitation**, not a code defect — correctly recorded.

- **No bug masked by external_body:** the two external_body helpers carry
  meaningful `requires`/`ensures` (not empty), and their logic is simple,
  correct iteration — no defect is being hidden behind the trust boundary.
- **No unrecorded verification failure:** module verify exit 0; phys-mod admit=0.

---

## 9. Issues (priority-ordered)

**Blockers:** NONE.

**Advisory nits (non-blocking):**
1. `init` Ok-arm does not pin the exact post-state (composed
   `seed(..).book_all(P).book_covered(M)` dropped) → an over-reserving impl would
   pass. Conservative/safe; no caller safety lost. (view_design §4.4)
2. `init` static one-shot `requires !initialized` dropped → one-shot is enforced
   dynamically only (frame::init Errs on 2nd call). Sound, more permissive.
3. `init` Err arm promises only `inv()` (can be vacuous if `!initialized`); could
   assert `wf` is preserved once `frame::init` succeeded. Caller observes no
   partial state, so acceptable.
4. `book_physical_memory_regions` Err arm dropped the view_design `!all_free(R)`
   fail-fast conflict predicate. Caller doesn't consume it.
5. MMIO "uncovered frame unchanged" half not stated (only covered⟹reserved).
   Caller is told not to rely on uncovered frames.
6. `ExLinkedList` external_type_specification has no dedicated allowed-list line
   in tcb-allowed.md (justified only in the `book_*` prose). Documentation
   tidiness; the mechanism is skill-sanctioned.

None of items 1–6 weakens a guarantee the sole caller (`kernel_vas::init`)
actually consumes, and none introduces unsoundness.

---

## 10. Final Verdict: **PASS**

Justification:
- phys-mod own counts: **admit=0, assume=0** (no guardrail blocker).
- The **2** phys-mod `external_body` (`book_physical_memory_regions`,
  `book_mmio_regions`) are **both pre-listed in tcb-allowed.md**, justified by the
  documented (orphan-rule) LinkedList limitation. `init` is body-verified.
- `ExLinkedList` external_type_specification is the prescribed mechanism for a
  foreign std type; counted separately from external_body; not a violation.
- All **4** `uninterp spec fn` qualify as mechanical consequences of external
  boundaries (protected raw-memory accessor; singleton-global ghost View
  accessor backed by the TCB `frame::instance`; two foreign-type frame-set
  accessors over the un-modelable LinkedList). FIRM VERDICT: acceptable, not a
  verification escape.
- `Err(_) => true` is paired with an unconditional `inv()` (and `initialized` for
  the helpers). FIRM VERDICT: meaningful, caller-justified error path — NOT a
  one-sided/tautological-error violation.
- Caller coverage **10/10** caller-relied-upon properties; no missing guarantee.
- AST consistent (4/4, 0 mismatch), no `// VERUS REWRITE`, no contract drift.
- `make verify-kernel MODULE=mm::phys` exit **0**, zero verification errors.
- Single bugs.md entry reconciles; no bug masked by external_body; no unrecorded
  failure.

No genuine blocker exists. Remaining items are advisory spec-richness/tidiness
nits, all deliberately reduced and caller-justified. **PASS.**
