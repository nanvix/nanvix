# Spec Quality

Status: **FAIL**

Reviewed in-scope contracts (`UserFrame::{new,address,leak,share,refcount,drop}`, `Upool::{new,alloc}`) against `spec-design` criteria.

- **BLOCKER:** `UserFrame::drop` has no behavioral `ensures` (only `opens_invariants none` / `no_unwind`). Caller-required release semantics (release one ref; free on last ref) are not specified.
- **BLOCKER:** `UserFrame::share` omits caller-critical transition semantics: no `+1` refcount postcondition on `Ok`, and no explicit state-preservation/no-new-ref postcondition on `Err`.
- `UserFrame::{new,address,leak,refcount}` capture address/value facts, but do not state explicit global frame-condition guarantees (no allocation/no refcount mutation/no free) that callers rely on.
- Non-blocking spec hygiene: several ensures are subsumed/redundant (`result.inv()` in `new/address/leak`, `uf.inv()` in `share`) given equality + preconditions.
- `Upool::new` and `Upool::alloc` contracts are clear; `Upool::alloc` has a strong two-arm `match` including error-path semantics.

No tautological `Err(_) => true` clause found in upool contracts.

# Caller Coverage (Covered N/Total + Missing)

Status: **FAIL**

Mapped caller expectations from `caller_analysis.md` to existing requires/ensures.

**Covered 8 / 15**

Covered:
1. `UserFrame::new` preserves address identity (`result@ == addr@`).
2. `UserFrame::address` returns owned address (`result@ == self@`).
3. `UserFrame::leak` returns owned address (`result@ == self@`).
4. `UserFrame::share` success aliases same frame (`uf@ == self@`).
5. `UserFrame::refcount` success returns frame refcount value.
6. `Upool::new` establishes `result@.wf()`.
7. `Upool::alloc` success matches `alloc_one` transition.
8. `Upool::alloc` error preserves pool + requires exhaustion.

Missing:
1. `UserFrame::new`: explicit no-allocation/no-refcount-change contract.
2. `UserFrame::address`: explicit no-side-effect contract.
3. `UserFrame::leak`: explicit suppress-drop/no-free contract.
4. `UserFrame::share` (`Ok`): explicit refcount `+1` transition.
5. `UserFrame::share` (`Err`): explicit no-new-ref / unchanged-frame-state contract.
6. `UserFrame::refcount`: explicit pure-read/no-mutation contract.
7. `UserFrame::drop`: explicit release semantics (`release` / last-ref free).

Refcount-transition semantics (`share +1`, `drop release`) are currently deferred (as documented in `bugs.md`/`view_design.md`), but from strict caller-contract coverage this remains an unresolved gap.

# Proof Completeness (admit count+locations, external_body count+locations)

Status: **PASS** (for in-scope upool files)

Files checked: `upool.rs`, `upool.spec.rs`, `upool.proof.rs`.

- `admit()` count: **0**
  - Locations: none.
- `external_body` attribute count: **2**
  - `src/kernel/src/mm/phys/upool.rs:246` — `Upool::new`
  - `src/kernel/src/mm/phys/upool.rs:272` — `Upool::alloc`

# TCB Compliance (PASS/FAIL)

Status: **PASS**

Both in-scope `external_body` functions are present in `verus-ai-logs/tcb-allowed.md`:
- `src/kernel/src/mm/phys/upool.rs::Upool::new`
- `src/kernel/src/mm/phys/upool.rs::Upool::alloc`

No new trust boundary introduced in upool files.

# Guardrails Compliance (exact counts)

Status: **PASS**

Exact counts in upool files (`upool.rs`, `upool.spec.rs`, `upool.proof.rs`):
- `admit`: **0**
- `assume`: **0**
- `external_body` (attribute form): **2**
- `assume_specification`: **0**
- cfg-gated exec (`#[cfg(not(verus_keep_ghost))]`): **1**
  - `src/kernel/src/mm/phys/upool.rs:205` guarding `error!(...)` logging in `drop`.
  - This matches the allowed logging exception.

# AST Consistency (PASS/FAIL)

Status: **PASS**

Checks performed:
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref HEAD src/kernel/src/mm/phys/upool.rs summary`
  - Result: all 8 in-scope exec functions and 2 structs `MATCH`, 0 mismatches.
- `// VERUS REWRITE` scan in upool files:
  - Count: **0**
  - Therefore no rewrite-equivalence mismatch found.

# Verification (PASS/FAIL + error count)

Status: **PASS**

Command run:
- `cd /home/ruize/nanvix-phy-specs && make verify-kernel MODULE=mm::phys`

Observed result:
- `verification results:: 42 verified, 0 errors (partial verification with --verify-*)`
- Exit code: **0**
- Error count: **0**

# Bug Summary

Reconciled against `verus-ai-logs/nanvix-phys-phys-upool/bugs.md`:

- Recorded claim "no true code bugs found" remains consistent with current exec code review of in-scope functions.
- Recorded deferred modeling note (global `phys_view()` limits old/new transition expression for `share`/`drop`) remains applicable.
- `Upool::new` and `Upool::alloc` remaining `external_body` status is still true and TCB-listed.

Classification (per bug-reporting skill):
- **True code bugs:** none identified.
- **Context-dependent / verification limitations:** unresolved contract-level deferral of refcount transitions and no-effect frame conditions for several `UserFrame` methods.
- **False positives:** none observed.

# Issues (highest priority first)

1. **BLOCKER — Missing `drop` release contract**
   - `UserFrame::drop` lacks ensures for releasing one reference / last-ref reclaim semantics required by callers.
2. **BLOCKER — `share` contract under-specifies aliasing transition**
   - No explicit `+1` refcount guarantee on success; no explicit unchanged-state guarantee on error.
3. **BLOCKER — Caller expectation coverage incomplete (8/15)**
   - Multiple caller-relied semantic guarantees are absent from current requires/ensures.

# Result (PASS/FAIL)

**FAIL**

Strict final-review criteria are not fully met due to the blocker-level specification and caller-coverage gaps above.

Spec drift check command requested by prompt was run:
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff /home/ruize/nanvix-phy-specs/src/kernel/src/mm/phys/upool.rs --before HEAD`
- Result: **No contract drift detected** (no original guarantee weakened).
