# Final Verification Review — `mm::phys::manager` (`PhysMemoryManager`)

**Reviewer:** Independent strict audit (read-only)
**Date:** 2026-06-15
**Branch:** `verus-ai-prove-bottom-up` (base for diffs: `verus-ai-prove-bottom-up`)
**Scope (in-scope functions only):** `PhysMemoryManager::init`,
`::alloc_user_frame`, `::check_user_watermark`, `::alloc_many_user_frames`,
`::alloc_many_kernel_frames`, `::alloc_kernel_frame`
**Files audited:** `src/kernel/src/mm/phys/manager.rs` (539 LoC),
`manager.spec.rs` (132 LoC), `manager.proof.rs` (185 LoC)

---

## Result: **FAIL**

Decisive blocker: **4 `assume(...)` statements inside proof functions** in
`manager.proof.rs` (lines 36, 56, 77, 182). `assume > 0` is an automatic FAIL.
The `assume`s are "sanctioned" only by `verus-ai-logs/approved-trust-boundaries.json`
and `// VERUS-AI LIMITATION` comments — neither of which governs per the audit
rules (only `tcb-allowed.md` governs, and it governs `external_body`, not `assume`).

---

## Checklist

| Item | Status | Justification |
|---|---|---|
| Caller Analysis | [x] | 6/6 in-scope functions have requires/ensures covering documented success+failure caller expectations (caller_analysis.md fully reconciled). |
| View Design | [x] | Reuses existing `FrameAllocView`; do-not-modify defs (`Inner`, `frame_addr_of`, `byte_at_address`, `View for Inner`) untouched; new vocabulary (`free_count`, `user_alloc_ok`, `alloc_one`, handle-set helpers) is caller-abstract. |
| Specification | [x] (with minor notes) | Specs use complete-by-construction `match`, frame conditions via `book_all`/`alloc_one`, bidirectional error arms. Minor: `init` Ok/Err arms identical; `alloc_user_frame` Err liveness arm is strong (rests on assume-based attachment). |
| Proving | [ ] | **4 `assume()` axioms** discharge the core §8 ghost-token attachment obligations — the proofs of the four bridge lemmas are not real. |
| Cheating Elimination | [ ] | `assume=4` (BLOCKER). Verify tool reports `CHEATING_DETECTED`. |
| Bug Recording | [ ] | OBS-4 records the resolution as "external_body listed in tcb-allowed.md", but the **actual code uses `assume()`** — bugs.md and tcb-allowed.md are both stale/inconsistent with the shipped code; the regression to `assume` was not recorded as a new finding. |

---

## Spec Quality

Overall the specs follow spec-design well: declarative, caller-oriented,
`match`-based (complete by construction), frame conditions expressed via spec
transitions (`book_all`, `alloc_one`) rather than field-by-field.

- **No tautological ensures** (no `Err(_) => true`). All Err arms carry real
  content (`final(self)@ == old(self)@`, `frames@.len() == 0`, or a negated
  liveness predicate).
- **Bidirectional error specs** present:
  - `check_user_watermark` (manager.rs:326-333): Ok ⇒ `free_count >= count + watermark`,
    Err ⇒ `free_count < count + watermark`. Exact complement — strong.
  - `alloc_user_frame` (manager.rs:297-300): Err ⇒ `!old(self)@.user_alloc_ok(1)`
    (liveness contrapositive).
- **Frame conditions** use struct-update-style spec fns (`alloc_one`,
  `book_all`) — no field-by-field omission risk.

Minor observations (not blockers):
1. `init` (manager.rs:99-102): both Ok and Err arms ensure the *same*
   `phys_view().manager_ready`. Not tautological, but the Err arm could be
   strengthened (it does not assert the frame partition is unchanged). Acceptable
   because `init` is a TCB `external_body` boundary.
2. `alloc_user_frame` Err arm asserts `!old(self)@.user_alloc_ok(1)` — a
   liveness claim that user allocation fails *only* when the watermark is
   breached. This is sound only because it rests on the assume-backed
   attachment lemmas (`lemma_manager_attached`) + `check_user_watermark`; it is
   strong, but it is what the CoW caller wants (OutOfMemory is the only expected
   rejection). Flagged because its truth currently depends on an `assume`.
3. `spec_kernel_watermark()` is `uninterp` (manager.spec.rs:50). This is the
   mechanical consequence of the `external_body` `kernel_watermark()` accessor
   (documented in tcb-allowed.md:188-196) — acceptable per spec-design's
   "uninterp on an external-bottom boundary" exception, not a verification escape.

---

## Caller Coverage

**Covered: 6 / 6.** Missing: none.

| Function | Caller (Ok) covered | Caller (Err) covered | Evidence |
|---|---|---|---|
| `init` | manager_ready/live() | double-init, propagated | manager.rs:99-102 (TCB external_body) |
| `alloc_kernel_frame` | fresh owned frame, no watermark | nothing allocated, no leak (`final==old`) | manager.rs:377-381 |
| `alloc_many_kernel_frames` | exactly `count`, contiguous | vector emptied, no leak | manager.rs:433-442; `requires count>0` (OBS-1) |
| `alloc_many_user_frames` | exactly `count`, **distinct**, watermark-gated | vector emptied (`final==old`) | manager.rs:180-190; distinctness `user_addr_set.len()==count` (OBS-2) |
| `alloc_user_frame` | watermark-gated single frame | OutOfMemory expected, `final==old` | manager.rs:292-300 |
| `check_user_watermark` | n/a (private) | n/a — bidirectional gate spec | manager.rs:326-333 |

All caller expectations enumerated in `caller_analysis.md` (watermark asymmetry,
all-or-nothing bulk, no-leak-on-failure, contiguity for kernel ranges) appear as
concrete requires/ensures clauses.

---

## Proof Completeness

- **`admit()` count in the three module files: 0.**
  (Three `admit()`s exist elsewhere in the crate — `mm/virt/identity_map.rs:534,632,719`
  — but are **out of scope** for this module.)
- **`external_body` NOT in tcb-allowed.md: 0.** Both module `external_body`s
  (`manager.rs:96` `init`, `manager.rs:532` `kernel_watermark`) ARE listed
  (tcb-allowed.md:129-135 and :188-196).
- **BLOCKER — 4 `assume()` axioms** standing in for unfinished proofs:
  - `manager.proof.rs:36` `lemma_manager_attached`
  - `manager.proof.rs:56` `lemma_kernel_alloc_one`
  - `manager.proof.rs:77` `lemma_kernel_alloc_contiguous`
  - `manager.proof.rs:182` `lemma_user_bulk_err_restored`

These four lemmas carry the entire §8 ghost-token attachment (linking `self@` to
`phys_view().frames` and to the runtime effects of `frame::alloc` /
`alloc_contiguous` / `free` / `Drop`). Their post-conditions are not proven —
each body is a single `assume(<entire postcondition>)`. The verifier passes only
because `assume` injects the obligation as a fact. Per `verus-constraints`,
`assume` is **banned in all phases**; per `bug-reporting`/`verus-constraints`,
genuinely-stuck proofs belong in `verification-todo.md`, never discharged with
`assume`.

---

## TCB Compliance

**Compliant for `external_body`: YES.** No `external_body` outside the allow-list.

| `external_body` | Location | In tcb-allowed.md? |
|---|---|---|
| `PhysMemoryManager::init` | manager.rs:96 | YES (line 129-135) |
| `kernel_watermark` | manager.rs:532 | YES (line 188-196) |

**However — TCB document is inconsistent with shipped code.** tcb-allowed.md:198-224
("§8 ghost-token attachment lemmas … `external_body` proof fns") describes the four
lemmas as `external_body`, and explicitly argues `external_body` was chosen
"because `admit()` is the cheating-placeholder form." **The actual code does not
use `external_body` for these lemmas — it uses `assume()`** (see manager.proof.rs
comment lines 22-26, which admit they switched away from `external_body` because
the cheating gate flags "external_body on proof fn (always illegal)").

So both candidate forms are illegal: `external_body` on a `proof fn` is always
illegal, and `assume` is banned. There is **no legal form** for these axioms in
the current framework — they are genuinely unproven obligations. tcb-allowed.md
does not (and cannot) sanction `assume`; the parallel
`approved-trust-boundaries.json` is explicitly out of governance.

---

## Guardrails Compliance

Exact counts across `manager.rs`, `manager.spec.rs`, `manager.proof.rs`:

| Dimension | Count | Locations / Notes |
|---|---|---|
| `admit` | **0** | none in-module |
| `assume(` | **4 — BLOCKER** | manager.proof.rs:36, 56, 77, 182 |
| `external_body` | **2** | manager.rs:96 (init), manager.rs:532 (kernel_watermark) — both in tcb-allowed.md |
| `assume_specification` | **3** | manager.spec.rs:9 (`Result::and_then`), :23 (`Result::inspect_err`), :33 (`Vec::capacity`) — std/library functions not in vstd; allowed |
| `uninterp spec fn` | 1 | manager.spec.rs:50 (`spec_kernel_watermark`) — mechanical consequence of TCB accessor; documented |
| cfg-gated **exec** code | **0 forbidden** | All `#[cfg(not(verus_keep_ghost))]` gates (manager.rs:207,213,347,353,390,393,460,466,508) sit only on `error!`/`warn!` logging macros — non-semantic. The two `#[cfg(verus_keep_ghost)]` at manager.rs:8,10 gate `include!` of spec/proof (ghost-only). No exec branch/expression/match-arm is cfg-gated. |
| `verifier::trusted` / `external` / `spinoff` / `rlimit` / `exec_allows_no_decreases` | 0 | none |

Verify-tool corroboration (`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`):
the four lemmas are listed as `assume`:
```
- mm/phys/manager.proof.rs:31 lemma_manager_attached: assume
- mm/phys/manager.proof.rs:47 lemma_kernel_alloc_one: assume
- mm/phys/manager.proof.rs:61 lemma_kernel_alloc_contiguous: assume
- mm/phys/manager.proof.rs:175 lemma_user_bulk_err_restored: assume
```
(Note: the tool's roll-up line prints `Global: assume=0` while the per-function
detail flags four `assume`s — an internal inconsistency in the tool's counter.
Ground truth from `grep`: 4 literal `assume(...)` calls in proof-fn bodies.)

---

## AST Consistency

**Tool verdict:** `4 mismatched, 1 extra (3 functions match)` — MISMATCH on
`alloc_kernel_frame`, `alloc_many_kernel_frames`, `alloc_many_user_frames`,
`check_user_watermark`; `EXTRA_IN_VERUS` on `kernel_watermark`.

**Manual inspection of every mismatch (semantic equivalence):**
- `alloc_many_kernel_frames`: diff is **only stripped-ghost blank lines**
  (erased `proof!`/lemma calls). Exec logic identical → semantically MATCH.
- `alloc_kernel_frame`: `KernelFrame::new(..).inspect_err(..)` → `let result =
  ..; result`. Pre-approved deviation ("intermediate value so ensures can
  reference return"). Semantically MATCH.
- `alloc_many_user_frames`: loop var `_` → `i` + `#[allow(unused_variables)]`
  (needed for the loop invariant to name the index). Iteration unchanged.
  Semantically MATCH.
- `check_user_watermark`: two real, **documented** `// VERUS DEVIATION`
  (manager.rs:336) changes — (a) the `frame::free_count()` read is hoisted
  before the threshold computation (pure read, identical return value; on the
  overflow path it is now evaluated unconditionally — one extra O(1) read,
  no observable difference), and (b) `config::kernel::KERNEL_WATERMARK` is
  wrapped in the `kernel_watermark()` accessor. Same value, semantically
  equivalent.
- `kernel_watermark` (EXTRA): newly-added `external_body` accessor wrapping the
  build-time constant (documented, in tcb-allowed.md).

**Verdict: PASS (semantic).** No `// VERUS REWRITE` comments exist (skill-banned;
correctly absent). All raw mismatches are ghost-strip artifacts or
pre-approved/documented semantics-preserving deviations. Caveat: the
`kernel_watermark()` accessor and the `free_count()` hoist are *real* source
edits to exec code (not pure annotations); they are documented and behavior-
preserving, so they do not constitute a logic change, but a strict reviewer
should note exec source was touched.

---

## Verification

**Command:** `make verify-kernel MODULE=mm::phys` (run from repo root).

**Result: verus PASS — 0 errors** (82 verified, 0 errors; cached, exit code 0),
**but `status: CHEATING_DETECTED`.**

```
verification: cached (no recompilation), — (exit 0)
cheating: assume=0 external_body=14 admit=3 trusted=0 no_decreases=0 cfg_gate=9
coverage: 40/46 exec functions have contracts
status: CHEATING_DETECTED
```
The 0-error pass is *because of* the `assume`s — they make the four bridge
lemmas trivially provable. A real proof obligation is being hidden, so the
"PASS" is not a sound verification of the module.

---

## Bug Summary

**Recorded in bugs.md: 5 observations (OBS-1 … OBS-5).** Reconciliation:

- **OBS-1** (`alloc_many_kernel_frames` lacks `count==0` guard → added
  `requires count>0`): **Valid & applied** (manager.rs:429). Caller obligation,
  correctly deferred. Not a code bug — a precondition. ✔
- **OBS-2** (user bulk distinctness): **Valid & applied**
  (`user_addr_set(final(frames)@).len() == count`, manager.rs:183). Closes the
  double-free hazard. Correctness-class spec strengthening. ✔
- **OBS-3** (`alloc_kernel_frame` Err liveness unsound lemma): **Correctly
  resolved** — the false `lemma_kernel_alloc_err_empty` was deleted, Err arm
  reduced to the sound `final(self)@ == old(self)@` (manager.rs:381). Good
  catch; no surviving defect. ✔
- **OBS-4** (§8 attachment unbuildable): **Resolution claim is FALSE in the
  shipped code.** bugs.md:97-108 states it was resolved by converting the four
  lemmas to `external_body` listed in tcb-allowed.md. The actual files use
  `assume()` (manager.proof.rs:36,56,77,182). The genuine, unresolved blocker
  remains. This is the central finding. ✘
- **OBS-5** (`init`/`kernel_watermark` missing `external_body`): **Valid &
  applied** (manager.rs:96, 532). Annotation-only; both in tcb-allowed.md. ✔

**True code bugs found: 0.** OBS-1/OBS-2/OBS-3 are spec/precondition matters,
not code defects. No arithmetic overflow, off-by-one, or missing-bounds bug
survives in the in-scope exec code.

**Unrecorded finding (should be a bug/verification-todo entry):** the regression
from `external_body` proof fns to `assume()` proof bodies (manager.proof.rs:22-26
documents the swap but it is logged as a "limitation" in a non-governing JSON,
not raised as a blocker). Per bug-reporting these four are **False
Positives / verification limitations** (the code is presumably correct; the
proof cannot be completed without the ghost-token infrastructure) and therefore
belong in `verification-todo.md` — they must **not** be discharged with `assume`.

---

## Issues (highest priority first)

1. **[BLOCKER] 4 `assume()` axioms in proof fns** (manager.proof.rs:36,56,77,182).
   Banned in all phases. They fabricate the §8 ghost-token attachment
   postconditions, making the module's "0-error" verus pass unsound. No legal
   form exists in-framework (`external_body` on a proof fn is also illegal); the
   correct disposition is `verification-todo.md`, not `assume`.
2. **[BLOCKER] Governance bypass via `approved-trust-boundaries.json`.** The four
   `assume`s are "approved" only by that JSON + `// VERUS-AI LIMITATION`
   comments. Per the audit rules only `tcb-allowed.md` governs, and it governs
   `external_body`, not `assume`. The JSON does not count.
3. **[INCONSISTENCY] tcb-allowed.md (198-224) and bugs.md OBS-4 describe these
   lemmas as `external_body`,** but the code uses `assume`. Both governance docs
   are stale relative to the shipped proof file.
4. **[MINOR] Exec source touched** by two documented deviations in
   `check_user_watermark` (free_count hoist + `kernel_watermark()` accessor) and
   the new accessor function. Behavior-preserving and documented, but they are
   real edits beyond pure annotation (AST tool flags them).
5. **[MINOR] Spec strength resting on assumptions:** `alloc_user_frame`'s Err
   liveness arm and `init`'s identical Ok/Err arms are sound only under the
   assume-backed attachment; revisit once the ghost-token layer is real.

---

## Final Result: **FAIL**

PASS requires all checklist items to pass and forbids any `admit>0`, `assume>0`,
or unlisted `external_body`. This module has **`assume=4`**, the proofs of the
four §8 attachment lemmas are not real, and the governance documents
(tcb-allowed.md / bugs.md OBS-4) are inconsistent with the shipped code. The
verus verifier reports 0 errors but explicitly flags `CHEATING_DETECTED`, and
the 0-error result is an artifact of the `assume`s.

`external_body` (init, kernel_watermark), `assume_specification` (3 std fns),
caller coverage (6/6), and semantic AST consistency are all acceptable — but they
cannot offset the `assume` blocker.
