# Independent Strict Final Review — `mm::phys::manager` Verus effort

Target files reviewed:
- `src/kernel/src/mm/phys/manager.rs`
- `src/kernel/src/mm/phys/manager.spec.rs`
- `src/kernel/src/mm/phys/manager.proof.rs`
- `verus-ai-logs/nanvix-phys-phys-manager/{caller_analysis.md,view_design.md,bugs.md}`
- `verus-ai-logs/tcb-allowed.md`

In-scope functions:
`PhysMemoryManager::init`, `alloc_user_frame`, `check_user_watermark`, `alloc_many_user_frames`, `alloc_many_kernel_frames`, `alloc_kernel_frame`.

---

## 1) Spec quality review (external-top API contracts)

### Findings

### 1.1 Tautological / weak error-path specs (BLOCKER)
- `alloc_user_frame`: `Err(_) => true` (`manager.rs:264`).
- `check_user_watermark`: `Err(_) => true` (`manager.rs:303`).
- `alloc_kernel_frame`: `Err(_) => true` (`manager.rs:349`).

Per spec-design quality criteria, these are one-sided/tautological error arms and do not provide caller-usable guarantees about failure behavior.

### 1.2 Bulk error arms are only partially specified (BLOCKER)
- `alloc_many_user_frames`: `Err(_) => final(frames)@.len() == 0` (`manager.rs:195`).
- `alloc_many_kernel_frames`: `Err(_) => final(frames)@.len() == 0` (`manager.rs:406`).

This captures vector cleanup but not allocator rollback/no-leak/no-new-allocation facts expected by callers.

### 1.3 `init` contract does not encode caller-relevant lifecycle semantics (BLOCKER)
`init` only requires/ensures `phys_view().initialized` and `phys_view().inv()` (`manager.rs:101-106`). It does not distinguish `Ok`/`Err` behavior, does not specify duplicate-init failure conditions, and does not expose any postcondition equivalent to “singleton established for future `get_mut` use” expected by callers (`caller_analysis.md:53-61`).

### 1.4 Watermark predicate root is uninterpreted (policy concern, treated as BLOCKER)
`spec_kernel_watermark` is declared as `pub uninterp spec fn ...` (`manager.spec.rs:35`) with no linking constraint to `config::kernel::KERNEL_WATERMARK`. Under strict verus-constraints policy, uninterpreted spec functions are disallowed for this role; this weakens interpretability/auditability of the watermark contract.

### 1.5 Understandability
Contracts are readable and mostly well-commented (e.g., rationale blocks at `manager.rs:171-176`, `246-248`, `287-290`, `382-387`), but quality is insufficient due to missing/non-informative failure guarantees.

### 1.6 Spec-drift helper check
Command run:
`python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff /home/ruize/nanvix-phy/src/kernel/src/mm/phys/manager.rs --before HEAD`

Result: **PASS (no contract drift detected)**.

---

## 2) Caller coverage against `caller_analysis.md`

Method: extracted explicit caller expectations (success/failure + documented precondition behavior) from `caller_analysis.md:52-131`, then mapped to `requires/ensures` in `manager.rs`.

## Coverage summary
- **Fully covered:** 3
- **Partially covered:** 6
- **Missing:** 5
- **Covered N/Total (strict full coverage): 3/14**

## Missing / partial expectation mapping
1. **`init` success lifecycle guarantee** (`caller_analysis.md:53-55`) — **Missing**. No `Ok`-arm lifecycle postcondition (`manager.rs:99-106`).
2. **`init` error condition (already initialized / InvalidArgument)** (`56-58`) — **Missing**. No `Err`-arm condition in spec (`99-106`).
3. **`alloc_user_frame` success fresh/owned frame + watermark** (`66-70`) — **Partial**. Has allocated-membership/alignment/watermark (`258-263`), but no freshness/exclusive-ownership postcondition.
4. **`alloc_user_frame` failure no allocation/leak** (`72-73`) — **Missing** (`Err(_) => true`, `264`).
5. **`alloc_many_user_frames` precondition-validation behavior (empty vec + capacity checks yielding InvalidArgument)** (`82-84`) — **Partial**. Spec requires empty vec (`182`) but does not specify capacity requirement or InvalidArgument failure behavior.
6. **`alloc_many_user_frames` success exact count + watermark** (`85-87`) — **Covered** (`188-194`).
7. **`alloc_many_user_frames` failure all-or-nothing incl. no leaks** (`88-90`) — **Partial**. Only `final(frames).len()==0` (`195`), no allocator-state rollback guarantee.
8. **`alloc_kernel_frame` success owned frame, watermark bypass** (`97-100`) — **Partial**. Membership/alignment present (`345-348`), no explicit bypass/availability guarantee.
9. **`alloc_kernel_frame` failure no leak (including wrap failure path)** (`101-102`) — **Missing** (`Err(_) => true`, `349`).
10. **`alloc_many_kernel_frames` precondition-validation behavior** (`110-112`) — **Partial**. Requires empty vec (`393`) but no capacity requirement or InvalidArgument postcondition.
11. **`alloc_many_kernel_frames` success exact count contiguous run** (`113-115`) — **Covered** (`399-405`).
12. **`alloc_many_kernel_frames` failure all-or-nothing incl. no leaks** (`116-118`) — **Partial**. Only vector-empty guarantee (`406`).
13. **`check_user_watermark` semantics: `Ok` iff policy holds, pure gate** (`125-129`) — **Partial**. Only `Ok => spec_watermark_ok` (`302`), no converse (`Err => !predicate`) and no error classification.
14. **`check_user_watermark` error cases (overflow vs breach)** (`126-128`) — **Missing** (`Err(_) => true`, `303`).

---

## 3) Proof completeness

Checked `manager.proof.rs` and target files for residual placeholders.

- `admit()`: **0** (PASS)
- `assume(...)`: **0** (PASS)
- `external_body` remaining in in-scope module: **6** (all in `manager.rs`, expected trust shims)

`manager.proof.rs` contains two lemmas (`17-24`, `32-58`) and no admits.

**BLOCKER rule check:** “Any remaining admit() is BLOCKER” → satisfied (none).

---

## 4) TCB compliance

All `external_body` in `manager.rs` are present in approved list:
- Approved entries in `tcb-allowed.md:54-73` match:
  - `init` (`manager.rs:98/107`)
  - `alloc_user_frame` (`249/267`)
  - `check_user_watermark` (`292/306`)
  - `alloc_many_user_frames` (`177/198`)
  - `alloc_kernel_frame` (`336/352`)
  - `alloc_many_kernel_frames` (`388/409`)

## TCB result
- **Unapproved `external_body` in `manager.rs`: 0**
- **PASS** for item 4.

---

## 5) AST consistency + `VERUS REWRITE` audit

Commands run:
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/kernel/src/mm/phys/manager.rs count`
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/kernel/src/mm/phys/manager.rs summary`

Results:
- `Consistent: ✅ YES (matched=7 mismatched=0 missing=0 extra=0)`
- Functions and struct all `MATCH`.

`VERUS REWRITE` scan over module (`src/kernel/src/mm/phys`): **0 matches**.

## AST consistency result
- **PASS**

---

## 6) Verification run

Command run from repo root:
`make verify-kernel MODULE=mm::phys`

Observed output/log:
- Exit code: **0** (`verus-ai-logs/verify-kernel/verus-logs/verus_2026-06-15_04-26-24.log:6`)
- Mode: cached/no recompilation (`:7`, `:19`)
- Verus error count: **0** (no verification errors reported)

## Verification result
- **PASS** (with note: run reports global cheating inventory in module, expected for current trust-boundary state).

---

## 7) Guardrails compliance (exact counts + locations)

Scope for counting: `manager.rs`, `manager.spec.rs`, `manager.proof.rs` only.

(Computed via token scan ignoring comments + manual line verification.)

| Dimension | Count | Locations |
|---|---:|---|
| `admit` | 0 | None |
| `assume` | 0 | None |
| `external_body` | 6 | `manager.rs:98, 177, 249, 292, 336, 388` |
| `assume_specification` | 0 | None |
| `cfg-gated exec` (`cfg(not(verus_keep_ghost))` on exec code) | 0 | None |

Notes:
- `manager.rs` has `#[cfg(verus_keep_ghost)] include!(...)` at lines `9,11` and `cfg_attr(...allow(...))` at `97,291`; these are not `cfg(not(verus_keep_ghost))` exec-logic forks.

## Guardrails blocker check
- `admit > 0`? **No**.
- `assume > 0`? **No**.
- `external_body` not in approved TCB? **No**.

---

## 8) Bug reconciliation (`bugs.md`)

`bugs.md` content: “None” (`bugs.md:1-5`), plus non-bug notes (`7-25`).

Reconciliation:
- Existing bug entries: **None**.
- New real code defects found in this review: **None confirmed**.
- New findings are specification-quality/coverage defects (verification artifact quality), not proven runtime code defects.

Classification (per bug-reporting skill):
- No new **True Bug** entry produced.
- Contract weakness findings are **not** logged as code bugs.

---

## Helper checks

1. Function coverage:
- Command: `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/fn_coverage.py src/kernel/src/mm/phys/manager.rs src/kernel/src/mm/phys/manager.rs`
- Result: `Matched 7/7`, missing `0`.

2. Spec drift:
- Command: `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff /home/ruize/nanvix-phy/src/kernel/src/mm/phys/manager.rs --before HEAD`
- Result: **No contract drift detected**.

---

## Prioritized Issues

## P0 (BLOCKER)
1. Tautological `Err(_) => true` in three in-scope APIs (`alloc_user_frame`, `check_user_watermark`, `alloc_kernel_frame`) at `manager.rs:264, 303, 349`.
2. Missing caller-critical failure guarantees (especially no-leak/rollback semantics) for bulk and single allocators (`manager.rs:195, 406` plus missing conditions on tautological Err arms).
3. `init` contract does not encode caller-required lifecycle/error semantics (`manager.rs:99-106` vs expectations in `caller_analysis.md:53-61`).
4. Use of `uninterp spec fn spec_kernel_watermark` (`manager.spec.rs:35`) violates strict verus-constraints policy and reduces spec auditability.

## P1 (Major)
5. Caller expectation coverage is low under strict interpretation: **3/14 fully covered**; many only partial.

---

## Final Result

**FAIL**

Rationale: Verification command passes and TCB/AST/admit checks pass, but there are blocking spec-quality and caller-coverage deficiencies (tautological error arms, insufficient failure-path contracts, missing lifecycle semantics for `init`, and strict-policy issue with uninterpreted watermark spec).
