# Final Independent Review — `mm::phys::frame`

## Spec Quality

Evidence reviewed:
- `frame.rs`, `frame.spec.rs`, `frame.proof.rs` (direct file inspection)
- `spec_drift.py` report: **No contract drift detected**.

Findings:
- `Inner::*` contracts for in-scope methods are strong and stateful (explicit `old(self)@ -> final(self)@` transitions).
- API/free-function layer (`pub(super)` wrappers) is weaker than `Inner::*` for several paths.
- Tautological/weak clauses found:
  - `frame.rs:1529` (`alloc_contiguous`): `Err(_) => true`.
  - `frame.rs:1566-1567` (`free`): `ensures true`.
- Error-path framing (`state unchanged`) is absent in several wrapper contracts where caller analysis expects it.

Assessment: **Partially complete at API layer** (good inner specs; wrapper specs include weak/tautological error guarantees).

## Caller Coverage (Covered N/Total + Missing)

Source: `caller_analysis.md` expectation sections per function.

Counting rule used: each function-level success/failure expectation block in `caller_analysis.md` counted once (plus query semantics for `is_covered` and `free_count`).

- **Covered: 6 / 15**
- **Missing or only partially captured: 9 / 15**

Covered (examples):
- `alloc_range` Ok reserved-region guarantee (`frame.rs:1627-1630`).
- `book` Ok reserved-frame guarantee (`frame.rs:1608-1610`).
- `free` destructor constraints (`opens_invariants none`, `no_unwind`) (`frame.rs:1568-1569`).
- `share` Err disjunction (`frame.rs:1650-1652`).
- `refcount` Ok/Err (`frame.rs:1668-1673`).

Missing / partial:
1. `alloc` Ok lacks explicit wrapper-level free→allocated transition/refcount=1 (wrapper only says allocated contains frame).
2. `alloc` Err lacks wrapper-level unchanged-state guarantee.
3. `alloc_contiguous` Ok lacks wrapper-level all-frames-booked/refcount=1 guarantee.
4. `alloc_contiguous` Err is tautological (`Err(_) => true`).
5. `alloc_range` Err lacks unchanged-state guarantee.
6. `book` Err lacks unchanged-state guarantee.
7. `share` Ok lacks explicit wrapper-level refcount increment guarantee.
8. `is_covered` lacks explicit non-mutation guarantee (returns coverage equivalence only).
9. `free_count` lacks explicit non-mutation guarantee (value relation only).

## Proof Completeness (admit count + locations, external_body-not-in-TCB)

Tool-backed guardrail scan (comments stripped):
- `admit`: **0** (no locations)
- `assume`: **0**
- `assume_specification`: **0**

`external_body` in frame module files:
- Count: **10** (all in `frame.rs`)
- Locations: `1401, 1435, 1491, 1519, 1548, 1564, 1603, 1622, 1643, 1662`
- Functions mapped: `instance, init, alloc, alloc_contiguous, free_count, free, book, alloc_range, share, refcount`

`external_body` not in TCB allowlist: **0**.

## TCB Compliance (YES/NO + list)

**YES**.

All frame-module `external_body` functions are present in `tcb-allowed.md`:
- `frame.rs::instance`
- `frame.rs::init` (skip/excluded target)
- `frame.rs::alloc`
- `frame.rs::alloc_contiguous`
- `frame.rs::free_count`
- `frame.rs::free`
- `frame.rs::book`
- `frame.rs::alloc_range`
- `frame.rs::share`
- `frame.rs::refcount`

## Guardrails Compliance (exact counts)

### Frame module files only (`frame.rs`, `frame.spec.rs`, `frame.proof.rs`)
- `admit`: **0**
- `assume`: **0**
- `assume_specification`: **0**
- `external_body`: **10**
- cfg-gated exec code (`#[cfg(not(verus_keep_ghost))]`): **28**
  - locations in `frame.rs`: `167, 186, 203, 236, 303, 315, 358, 408, 491, 580, 594, 646, 749, 764, 796, 862, 886, 899, 974, 995, 1179, 1182, 1219, 1221, 1223, 1226, 1290, 1294`

### Global kernel counts (same scan method over `src/kernel/src`)
- `admit`: **16**
- `assume`: **0**
- `assume_specification`: **10**
- `external_body`: **17**
- cfg-gated exec code (`#[cfg(not(verus_keep_ghost))]`): **45**

Blocker check from guardrails: `admit==0` and `assume==0` in frame module ✅

## AST Consistency (PASS/FAIL)

- `VERUS REWRITE` comments in frame module files: **none found**.
- `VERUS BUG FIX` comments found at `frame.rs:297, 547, 716, 851, 856, 951, 1023, 1085`.
- Each bug-fix site is implemented in code as claimed:
  - `alloc_contiguous` has `count > num_bits` guard before bitmap range alloc.
  - top-of-space unwrap panic avoided by total division-based index in `free/share/refcount/book/is_covered/alloc_range`.
  - unaligned input rejected in `Inner::refcount`.

Assessment: **PASS** (no rewrite mismatch found; bug-fix comments match implementation).

## Verification (PASS/FAIL + error count)

Commands executed:
1. `make verify-kernel MODULE=mm::phys`
   - Exit code: **0**
   - Verifier reported no proof errors for module verification run.
2. `make verify` (cross-module regression)
   - Exit code: **0**

Assessment: **PASS**, verification error count observed: **0**.

## Bug Summary (total + severities + reconciliation)

`bugs.md` entries reviewed: **5**

Reconciliation:
1. **[open] alloc_range off-by-one** → **Fixed in current code**.
   - Current implementation uses half-open range (`index < end_exclusive`, `for start..end_exclusive`) and no `admit`.
2. **[auto-fixed] top-of-space unwrap panic** → **Fixed** at all listed sites.
3. **[auto-fixed] weak representability invariant** → **Fixed** (`internal_inv` includes `i <= spec_max_frame_number()`).
4. **[auto-fixed] missing `count <= num_bits` guard in `alloc_contiguous`** → **Fixed**.
5. **[auto-fixed] diagnostic overflow in `alloc_range`** (saturating diagnostics) → **Fixed**.

Unrecorded bug found: **None blocker-level**. API-wrapper spec weakness noted separately under Issues.

## Issues (priority order)

### P2 — Wrapper-spec weakness / incompleteness (non-blocker under requested blocker rubric)
- `alloc_contiguous` error contract is tautological (`Err(_) => true`).
- `free` contract is tautological (`ensures true`) by design, but very weak.
- Several wrappers do not encode unchanged-state on `Err` despite caller-analysis expectations (`alloc`, `alloc_range`, `book`, etc.).

### P3 — Skill-file availability mismatch
- Requested skill files (`spec-design`, `verus-constraints`, `ast-consistency`, `bug-reporting`, `spec-completeness`, `spec-drift-check`) were not present under `.github/skills/`; only project skills listed by `ls` were available.

## Result: PASS

Reason: no blockers per required rubric:
- `admit == 0` ✅
- `assume == 0` ✅
- all frame-module `external_body` entries are TCB-allowed ✅
- AST consistency checks passed ✅
- verification commands passed (exit 0) ✅
