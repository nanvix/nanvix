# Final Independent Verification Review — `mm::phys::manager`

## Checklist

- [x] **Caller Analysis** — Reviewed `caller_analysis.md` and mapped expectations to contracts; coverage computed below (8/13 covered).
- [x] **View Design** — Reviewed `view_design.md`; manager uses `FrameAllocView` and does not modify do-not-touch view defs.
- [ ] **Specification** — External-top contracts are incomplete/weak on multiple failure/error-code obligations (details below).
- [ ] **Proving** — No `admit()`, but proof file contains 4 `assume(...)` escapes (blocker).
- [ ] **Cheating Elimination** — Guardrail blockers present (`assume` > 0 in `manager.proof.rs`).
- [ ] **Bug Recording** — `bugs.md` reconciliation is stale for OBS-4 and does not record current `assume(...)` blocker explicitly.

## Spec Quality

### In-scope functions reviewed
- `PhysMemoryManager::init` (`src/kernel/src/mm/phys/manager.rs:96-116`)
- `PhysMemoryManager::alloc_user_frame` (`manager.rs:286-309`)
- `PhysMemoryManager::check_user_watermark` (`manager.rs:326-358`)
- `PhysMemoryManager::alloc_many_user_frames` (`manager.rs:173-271`)
- `PhysMemoryManager::alloc_many_kernel_frames` (`manager.rs:425-521`)
- `PhysMemoryManager::alloc_kernel_frame` (`manager.rs:371-403`)

### Findings
1. **Tautological/redundant split in `init` postcondition**: both `Ok` and `Err` arms assert the same fact (`phys_view().manager_ready`) (`manager.rs:99-102`). This is effectively one unconditional ensures and does not differentiate success/failure semantics.
2. **Missing failure-path precision in `init`**: no ensures for error code/condition (`InvalidArgument` on double-init), despite caller expectation (`caller_analysis.md:64-65`).
3. **`alloc_many_user_frames` under-specifies failure cause**: Err arm only states rollback + empty vec (`manager.rs:187-190`), but does not state watermark failure predicate or error code.
4. **`alloc_many_kernel_frames` and `alloc_many_user_frames` omit capacity contract in spec**: runtime checks exist (`manager.rs:211-216`, `464-469`), but no corresponding `requires`/failure ensures on capacity.
5. **`alloc_user_frame` failure code not constrained**: spec ties Err to `!user_alloc_ok(1)` (`manager.rs:297-300`) but does not constrain returned error kind.
6. **`check_user_watermark` is comparatively strong**: bidirectional threshold split is explicit (`manager.rs:328-333`).

## Caller Coverage (Covered 8/13)

Source of expectations: `verus-ai-logs/nanvix-phys-phys-manager/caller_analysis.md:55-157`.

| # | Caller expectation | Evidence in contracts | Status |
|---|---|---|---|
| 1 | `init` Ok => manager ready/live | `manager.rs:100` | Covered |
| 2 | `init` Err => `InvalidArgument` on double-init | No error-code/condition ensures in `manager.rs:99-103` | **Missing** |
| 3 | `alloc_kernel_frame` Ok => one free frame becomes owned/reserved | `manager.rs:377-380` | Covered |
| 4 | `alloc_kernel_frame` Err => no leak/state unchanged | `manager.rs:381` | Covered |
| 5 | `alloc_many_kernel_frames` Ok => `count` contiguous frames | `manager.rs:434-436` | Covered |
| 6 | `alloc_many_kernel_frames` Err => vec empty + rollback | `manager.rs:439-442` | Covered |
| 7 | `alloc_many_kernel_frames` caller storage contract includes capacity >= count | `requires` lacks capacity (`manager.rs:426-430`) | **Missing** |
| 8 | `alloc_many_user_frames` Ok => `count` frames, uniqueness/non-contiguity acceptable | `manager.rs:182-185` | Covered |
| 9 | `alloc_many_user_frames` Err => vec empty + watermark rejection semantics | vec-empty covered (`manager.rs:189`), watermark rejection not specified in Err arm | **Missing** |
| 10 | `alloc_many_user_frames` caller storage contract includes capacity >= count | `requires` lacks capacity (`manager.rs:174-177`) | **Missing** |
| 11 | `alloc_user_frame` Ok => one frame + watermark gate | `manager.rs:293-296` | Covered |
| 12 | `alloc_user_frame` Err => no allocation | `manager.rs:298` | Covered |
| 13 | `check_user_watermark` embodies threshold gate used by both user paths | `manager.rs:329-333` | Covered |

## Proof Completeness

- **`admit()` count:** **0** in
  - `src/kernel/src/mm/phys/manager.rs`
  - `src/kernel/src/mm/phys/manager.spec.rs`
  - `src/kernel/src/mm/phys/manager.proof.rs`
  
  Evidence: `rg -n '^[[:space:]]*admit!?\(' ...` returned no matches.

- **`external_body` count (these 3 files):** **2**
  - `manager.rs:96` (`PhysMemoryManager::init`)
  - `manager.rs:532` (`kernel_watermark`)

- **`external_body` NOT in TCB:** **0**

## TCB Compliance

**YES** for `external_body` entries in this module.

Mapped entries:
- `manager.rs:96` `PhysMemoryManager::init` ↔ listed at `tcb-allowed.md:129`
- `manager.rs:532` `kernel_watermark` ↔ listed at `tcb-allowed.md:188`

No additional `external_body` attributes found in `manager.spec.rs`/`manager.proof.rs`.

## Guardrails Compliance

Counts across `manager.rs`, `manager.spec.rs`, `manager.proof.rs`:
- `admit`: **0**
- `assume`: **4** (**BLOCKER**)
  - `manager.proof.rs:36`
  - `manager.proof.rs:56`
  - `manager.proof.rs:77`
  - `manager.proof.rs:182`
- `external_body`: **2**
  - `manager.rs:96`, `manager.rs:532`
- `assume_specification`: **3**
  - `manager.spec.rs:9` (`Result::and_then`)
  - `manager.spec.rs:23` (`Result::inspect_err`)
  - `manager.spec.rs:33` (`Vec::capacity`)
- `cfg-gated exec code`: **0 forbidden**
  - Total cfg uses: `#[cfg(verus_keep_ghost)]` at `manager.rs:8,10` (ghost includes), and `#[cfg(not(verus_keep_ghost))]` at `manager.rs:207,213,347,353,390,393,460,466,508` (logging only: `error!`/`warn!`).
  - No cfg-gated branches/expressions altering runtime behavior detected.

## AST Consistency

**PASS**

Commands run:
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref verus-ai-prove-bottom-up src/kernel/src/mm/phys/manager.rs count`
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref verus-ai-prove-bottom-up src/kernel/src/mm/phys/manager.rs summary`

Evidence:
- `✅ Consistent: 8 functions, 1 structs match.`
- `Consistent: ✅ YES (matched=8 mismatched=0 missing=0 extra=0)`

`// VERUS REWRITE` inspection:
- `rg -n "VERUS REWRITE" src/kernel/src/mm/phys/manager.rs src/kernel/src/mm/phys/manager.spec.rs src/kernel/src/mm/phys/manager.proof.rs`
- No matches.

## Verification

**Command (exact):**
- `make verify-kernel MODULE=mm::phys`

**Result:**
- Verus run exit code: **0** (`verus-ai-logs/verify-kernel/verus-logs/verus_2026-06-15_14-50-50.log:6`)
- Verification error count: **0** (no `error` lines; `error_lines=0` from log scan)
- Wrapper summary still reports: **`status: CHEATING_DETECTED`** (due module-wide guardrail findings)

## Bug Summary

Recorded entries in `bugs.md`: **5** (`OBS-1`..`OBS-5`).

### Reconciliation
1. **OBS-1** (`bugs.md:5`) — **Still valid (Context-Dependent)**
   - `alloc_many_kernel_frames` still has no `count==0` fast path; call reaches `frame::alloc_contiguous(count)` (`manager.rs:471`) while spec requires `count > 0` (`manager.rs:429`).
2. **OBS-2** (`bugs.md:22`) — **Still valid (Context-Dependent)**
   - Distinctness guarantee remains spec-level (`manager.rs:183`) and depends on allocator non-aliasing assumptions.
3. **OBS-3** (`bugs.md:41`) — **Fixed**
   - Unsound `free_count()==0` Err claim removed; current Err clause is rollback (`manager.rs:381`).
4. **OBS-4** (`bugs.md:83`) — **Stale / not truly resolved under current guardrails**
   - File currently uses `assume(...)` in 4 proof fns (`manager.proof.rs:36,56,77,182`). Under this review policy, these are blockers.
5. **OBS-5** (`bugs.md:110`) — **Fixed**
   - `init` and `kernel_watermark` carry `#[verus_verify(external_body)]` (`manager.rs:96`, `532`).

### New unrecorded blocker found during this review
- **Verification-integrity blocker (Context-Dependent, high severity):** direct `assume(...)` escapes in `manager.proof.rs` (`36,56,77,182`).
- Not explicitly recorded in `bugs.md` as an active blocker (OBS-4 claims resolved in a different form).

### True bugs (active)
- **0 active true runtime code defects** identified in this read-only review.

## Issues (highest priority first)

1. **BLOCKER:** `assume(...)` present in proof file (`manager.proof.rs:36,56,77,182`).
2. **Spec contract gap:** missing failure/error-code precision for `init` (`manager.rs:99-103`) vs caller expectation (`caller_analysis.md:64-65`).
3. **Spec contract gap:** `alloc_many_user_frames` Err arm lacks watermark failure condition/error semantics (`manager.rs:187-190`, `caller_analysis.md:105-107`).
4. **Spec contract gap:** bulk alloc specs do not encode caller capacity contract (`manager.rs:174-177`, `426-430`, `caller_analysis.md:96-97`, `110`).
5. **Documentation drift:** `tcb-allowed.md` still describes manager proof lemmas as `external_body` (`tcb-allowed.md:198-224`) while file uses `assume(...)`.

## Result: **FAIL**

Rationale: per required guardrails, any `assume > 0` is a blocker. This module has `assume=4` in `manager.proof.rs`.
