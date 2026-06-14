# Final Verification Review — `mm::phys::manager` (PhysMemoryManager)

Independent strict review. Branch: `verus-ai-prove`. Date: 2026-06-15.
All findings verified with tools (grep, ast_consistency.py, spec_drift.py,
fn_coverage.py, git, /tmp/verify_mmphys.log, pipeline_state.json).

**RESULT: FAIL** — 4 `admit()` remain (admit>0 ⇒ FAIL), plus `init`
`external_body` masks an in-scope target function, plus AST source-integrity
mismatches. The pipeline itself records proving = `dialogue-BLOCKED` and
cheating-elimination = `FAIL-cheating`.

---

## Spec Quality

Five of the six target functions have strong, caller-oriented, `match`-based
specs with proper frame conditions and bidirectional error paths:

- `alloc_user_frame` (manager.rs:280-296): Ok = `user_alloc_ok(1)` ∧
  `free_frames.contains(uf@)` ∧ `final@ == old@.alloc_one(uf@)`; Err =
  `final@ == old@` ∧ `!user_alloc_ok(1)`. Strong, bidirectional. ✅
- `check_user_watermark` (manager.rs:320-327): Ok/Err bidirectional on
  `free_count() >= count + spec_kernel_watermark()`. ✅
- `alloc_kernel_frame` (manager.rs:363-374): Ok = `free_frames.contains(kf@)` ∧
  `final@ == old@.alloc_one(kf@)`; Err = `final@ == old@`. Correctly omits any
  watermark clause (kernel bypasses). ✅
- `alloc_many_kernel_frames` (manager.rs:417-435): Ok = `len==count` ∧
  `kernel_frames_contiguous` ∧ `all_free` ∧ `book_all`; Err = `final@==old@` ∧
  `len==0`; `requires count>0` (OBS-1). Complete. ✅
- `alloc_many_user_frames` (manager.rs:173-191): Ok includes the
  **distinctness** guarantee `user_addr_set(final(frames)@).len() == count`
  (OBS-2, closes the double-free hazard) plus `all_free`/`book_all`; Err =
  `final@==old@` ∧ `len==0`. Strong. ✅

**Weak spec — `init` (manager.rs:96-116):** Both `match` arms are identical
(`phys_view().manager_ready` on Ok and Err), so the `match` is degenerate — the
postcondition is effectively the unconditional `phys_view().manager_ready`. It
does **not** capture the documented Err semantics (double-init →
`InvalidArgument`) nor that the frame partition is untouched, and because the
function is `external_body` this postcondition is an *assumed* axiom, not proven.
This is a redundant-match / one-sided-error smell and is discussed further under
TCB Compliance. ⚠

No tautological or subsumed `ensures` found in the other five. No operational
(code-as-spec) clauses. `uninterp spec_kernel_watermark()` is intentional (see
Guardrails).

## Caller Coverage (Covered 6 / 6; Missing: none — minor weaknesses noted)

Mapped each expectation in `caller_analysis.md` to a requires/ensures:

| Function | Success expectation | Failure expectation | Verdict |
|---|---|---|---|
| `init` | manager ready / `live()` after Ok | double-init `InvalidArgument`, partition untouched | Ok-readiness covered; **Err not differentiated, partition-untouched not stated** (assumed via external_body) ⚠ |
| `alloc_kernel_frame` | fresh owning frame, drop frees, no watermark | nothing allocated, no leak | Covered ✅ |
| `alloc_many_kernel_frames` | exactly `count` **contiguous** owning frames | vec emptied, no leak, watermark bypassed | Covered ✅ |
| `alloc_many_user_frames` | exactly `count` **distinct** frames, watermark-gated | vec emptied, `OutOfMemory` on watermark | Covered ✅ (Err arm asserts `final@==old@`+`len==0`; does not assert `!user_alloc_ok`, justified: Err may also be a per-frame pool failure) |
| `alloc_user_frame` | one frame, same watermark as bulk | nothing allocated, `OutOfMemory` | Covered ✅ (Err asserts `!user_alloc_ok(1)`) |
| `check_user_watermark` (private) | `free>=count+watermark` | `free<count+watermark` | Covered ✅ |

Net: all six map. The only substantive gap is `init`'s degenerate Err arm, but
since `init` is `external_body` the contract is assumed regardless.

## Proof Completeness — BLOCKER

**`admit()` count: 4** (any admit ⇒ FAIL). Exact locations:

- `manager.proof.rs:16` — `lemma_manager_attached` (`m@ == phys_view().frames`)
- `manager.proof.rs:35` — `lemma_kernel_alloc_one`
- `manager.proof.rs:55` — `lemma_kernel_alloc_contiguous`
- `manager.proof.rs:216` — `lemma_user_bulk_err_restored`

These four are `pub proof fn` bodies reduced to `admit()`. They discharge the
§8 ghost-token attachment and `Drop`-based partition restoration that the exec
specs depend on (e.g. `alloc_kernel_frame`/`alloc_many_user_frames` rely on
them). Because they are admitted, every `ensures` of these lemmas is an unproven
axiom injected into the verified functions — the central correctness facts
(`self@ == phys_view().frames`, the alloc transitions, the error restoration)
are effectively assumed.

The skill (`verus-constraints`) requires genuinely-stuck proofs to be moved to
`verification-todo.md`; **no `verification-todo.md` exists** for this module
(only for `nanvix-phys-bump-allocator`). The admits are documented in `bugs.md`
but left as raw `admit()`, which the cheating gate fails.

**`external_body` count: 2** (in manager.rs):
- `manager.rs:96` (`#[verus_verify(external_body)]`) on `init` (body at :104)
- `manager.rs:524` (`#[verus_verify(external_body)]`) on `kernel_watermark` (body at :529)

## TCB Compliance

| `external_body` | Listed in tcb-allowed.md? | Rationale assessment |
|---|---|---|
| `manager.rs:524` `kernel_watermark` | **YES** (tcb-allowed.md:132-140) | **VALID.** Genuine external-bottom: `config::kernel::KERNEL_WATERMARK` is a build.rs-generated constant in a non-Verus dependency crate Verus cannot resolve. `ensures ret as nat == spec_kernel_watermark()`. Acceptable trust boundary. |
| `manager.rs:96` `init` | **YES** (tcb-allowed.md:86) | **STALE / INVALID — masking unverified in-scope code.** ⚠ BLOCKER-class |

**`init` assessment (critical).** tcb-allowed.md:86 lists `init` under
"Cross-module dependencies marked `external_body` (eliminated when their module
is verified)" with rationale *"no specs yet; opaque callee."* That rationale is
no longer true:

1. `init` is one of the **six in-scope target functions** of *this* module
   (`mm::phys::manager`) — it is not a cross-module dependency. `verus-constraints`
   states `external_body` on the current module's own functions is
   *"unconditionally forbidden in all phases — `admit()` ... is the correct
   placeholder."*
2. `init` now **has** a `#[verus_spec]` ensures (`phys_view().manager_ready`),
   contradicting "no specs yet." Marking it `external_body` turns that ensures
   into an unproven axiom and skips verification of the body (which writes the
   `static mut PHYS_MEMORY_MANAGER` behind an `AtomicBool`).

So the listing masks unverified, in-scope code. The correct handling is either
to verify the body (with a `PointsTo`/permission model for the `static mut`,
mirroring `frame::instance`'s pattern) or to use `admit()` in a proof obligation
— not `external_body`. As written this is a TCB-misuse blocker.

## Guardrails Compliance (exact counts, manager scope)

| Dimension | Count | Locations |
|---|---:|---|
| `admit()` | **4** | proof.rs:16, 35, 55, 216 — **BLOCKER** |
| `assume(...)` | 0 | — |
| `external_body` | 2 | manager.rs:96 (init), :524 (kernel_watermark) — init listing invalid (see TCB) |
| `assume_specification` | 3 | spec.rs:9 `Result::and_then`, :23 `Result::inspect_err`, :33 `Vec::capacity` — all std-lib, vstd-uncovered, allowed |
| `uninterp spec fn` | 1 | spec.rs:50 `spec_kernel_watermark` |
| cfg-gated **exec** (semantic) | 0 | all `#[cfg(not(verus_keep_ghost))]` gate only `error!`/`warn!` logging (manager.rs:207,213,339,345,382,385,452,458,500); `cfg_attr(verus_keep_ghost, verus_spec(...))` gates ghost loop invariants (:235,:465,:484); `cfg(verus_keep_ghost)` gates the spec/proof `include!` (:8,:10). None change exec semantics — permitted. |

Notes:
- The 3 `assume_specification` specs were inspected and are faithful to the std
  API (`and_then` forwards Err / applies op on Ok; `inspect_err` returns receiver
  unchanged; `Vec::capacity` opaque). Sound.
- `uninterp spec_kernel_watermark` (spec.rs:50) paired with the `external_body`
  `kernel_watermark()` accessor (`ensures ret as nat == spec_kernel_watermark()`)
  is, under the strict reading of `verus-constraints` / spec-design anti-pattern
  #12, the banned "uninterp + external_body axiom ≈ assume" shape. It is
  **justified and documented** (tcb-allowed.md:137-140) as a mechanical
  consequence of a genuine external-bottom build-time constant whose value
  callers never depend on. Not counted as a hard blocker per the task's blocker
  list, but flagged as a residual trust assumption.

## AST Consistency — FAIL

`ast_consistency.py summary`: matched=3, **mismatched=4**, extra=1.
True pre-verus baseline = `5e97f9a4f` (parent of first `verus_spec` commit
`54a1d5c94`). Per-function analysis against that baseline:

- **`check_user_watermark` — REAL EXEC CHANGE (substantive).** Original read
  `config::kernel::KERNEL_WATERMARK.checked_add(count)` and tested
  `frame::free_count() < watermark_threshold`. Verus version (a) replaced the
  constant with a new `kernel_watermark()` accessor call, and (b) hoisted
  `let available = frame::free_count()` **before** the `checked_add` overflow
  check, changing evaluation order. Documented in `bugs.md` as
  "behaviour-preserving," but it is a genuine source mutation not on the
  pre-approved deviation list. Per `verus-constraints`, an unresolvable constant
  should be handled by `#[verus_verify]` on the defining module, **not** by
  wrapping it in a new `external_body` accessor (which also adds a TCB element).
  The `free_count()` hoist is a proof-convenience reorder. ⚠
- **`kernel_watermark` — EXTRA_IN_VERUS.** New exec function added to the module
  (the accessor above). Adding exec functions is a source change. ⚠
- **`alloc_kernel_frame` — pre-approved deviation.** `KernelFrame::new(..).inspect_err(..)`
  returned directly → `let result = ...; result` (ensures must reference return
  value). Acceptable but lacks a `// VERUS DEVIATION` comment. (minor)
- **`alloc_many_user_frames` — documented BUILD-1.** `for _ in` → `for _idx in`
  (rename to name the index in the ghost invariant; `_idx` still unused in exec).
  Behavior-preserving, documented in `bugs.md`. (minor)
- **`alloc_many_kernel_frames` — benign.** Confirmed against baseline `5e97f9a4f`:
  exec tokens identical; the diff is purely stripped `proof!` blocks and
  cfg-gated logging. False mismatch. ✅

`alloc_user_frame`, `get_mut`, `init`, struct `PhysMemoryManager`: MATCH.

Verdict: **FAIL** — the `check_user_watermark` constant→accessor wrap +
`free_count` hoist and the added `kernel_watermark` function are real exec
deviations made to ease verification; the others are pre-approved/benign.

## Verification (from /tmp/verify_mmphys.log)

- Verus run: **cached (no recompilation), exit 0** — i.e. Verus reports no proof
  errors (`bugs.md` records "42 verified, 0 errors"). But this PASS is hollow:
  the 4 `admit()` lemmas pass vacuously and inject their `ensures` as axioms.
- Central gate status: **`CHEATING_DETECTED`**. Global cheating tally:
  `assume=0 external_body=18 admit=24 trusted=0 no_decreases=0 cfg_gate=15`.
  Manager contribution (cheating-detail.txt): 4 admit + 2 external_body.
- `pipeline_state.json`: proving `passed=False` (`dialogue-BLOCKED`),
  cheating-elimination `passed=False` (`FAIL-cheating`).

**Verification verdict: FAIL** (0 Verus proof errors, but gate = CHEATING_DETECTED
driven by the 4 manager admits; pipeline phases recorded as failed).

## Bug Summary

Recorded entries in `bugs.md`: 3 observations (OBS-1..3) + 1 build regression
(BUILD-1). True code bugs: **0**.

- **OBS-1** (`alloc_many_kernel_frames` lacks `count==0` guard): **still valid /
  resolved by spec** — `requires count > 0` added, matching the contiguous
  allocator. Obligation pushed to the (unverified) caller. Correct decision.
- **OBS-2** (user-bulk distinctness depends on allocator non-aliasing):
  **fixed** — `user_addr_set(final(frames)@).len()==count` in the Ok arm, proven
  by a real inductive loop proof (`user_bulk_inv`, `lemma_user_bulk_step`), no
  longer admitted. ✅
- **OBS-3** (`alloc_kernel_frame` Err liveness): **fixed in spec phase** — the
  unsound `lemma_kernel_alloc_err_empty` and its call sites were deleted; Err arm
  reduced to the sound `final@==old@`. Correct.
- **BUILD-1** (`unused variable i` under `-D warnings`): **fixed** — `_idx`
  rename. Auto-fix is appropriate.

Reconciliation: no regressions; no new true bug found. However, `bugs.md` frames
the 4 surviving `admit()`s as "irreducible within module scope" and leaves them
as `admit()`. Per `bug-reporting` / `verus-constraints`, genuinely-stuck proofs
must be hand-handed-off in `verification-todo.md` (which does not exist here) —
they are not accepted trust and they still fail the gate. Classification of the
4 admits: not bugs (False-Positive/trust-boundary class), but they are
unresolved verification failures that block PASS.

## Issues (highest priority first)

1. **[BLOCKER] 4 `admit()`** at proof.rs:16, 35, 55, 216. Any admit ⇒ FAIL. The
   core transition/attachment/restoration facts are assumed, not proven.
2. **[BLOCKER] `init` `external_body`** (manager.rs:96) on an in-scope target
   function that now carries a `#[verus_spec]` ensures. The tcb-allowed.md:86
   "no specs yet; opaque callee" rationale is stale; this masks unverified body
   + assumes the `manager_ready` postcondition. Forbidden by `verus-constraints`
   for the current module's own functions.
3. **[BLOCKER] AST source-integrity FAIL** — `check_user_watermark` real exec
   changes (constant→`kernel_watermark()` accessor; `free_count()` hoist) and the
   added `kernel_watermark` exec function. Constant should have used
   `#[verus_verify]` on the defining module per the skill, not a new external_body
   accessor.
4. **[Major] No `verification-todo.md`** for the 4 stuck admits — they are left
   as raw `admit()` instead of being honestly handed off.
5. **[Minor] `init` degenerate spec** — both `match` arms identical; Err
   (double-init) and partition-untouched not captured.
6. **[Minor] Residual trust shape** — `uninterp spec_kernel_watermark` +
   `external_body` accessor (justified, documented, but the banned uninterp+axiom
   pattern in strict reading).
7. **[Minor] Missing `// VERUS DEVIATION`** comment on `alloc_kernel_frame`'s
   `let result = ...; result` rewrite.

## Result: **FAIL**

Blockers present: `admit() = 4` (hard FAIL), `init` `external_body` masking an
in-scope target, and AST source-integrity mismatches. Verus's cached exit-0 is
not a genuine pass — the gate reports `CHEATING_DETECTED` and the pipeline
records both proving and cheating-elimination as failed.

**Exact counts (manager scope):** `admit=4` · `assume=0` · `external_body=2`
(init, kernel_watermark) · `assume_specification=3` · `uninterp=1` ·
semantic cfg-gated exec = 0.
