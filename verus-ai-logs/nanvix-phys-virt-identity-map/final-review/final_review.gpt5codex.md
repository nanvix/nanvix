# Final Independent Verification Review — `mm::virt::identity_map`

Scope reviewed (only): `identity_map_page`, `ensure_pt`, `ensure_pte`.
Files fully read: requested 8 files + target module/spec/proof files.

## Spec Quality

### `ensure_pt`
- **Requires:** only `identity_map_view().inv()`.
- **Ensures:** `Ok(pt_paddr) => inv + page-aligned(pt_paddr)`; `Err(_) => inv`.
- Assessment: non-tautological but **underpowered for callers**. Missing key caller-facing guarantees: PDE-present postcondition, returned `pt_paddr` corresponds to PDE, idempotence/unchanged-fast-path, and explicit failure framing beyond invariant preservation.

### `ensure_pte`
- **Requires:** only `identity_map_view().inv()`.
- **Ensures:** always `inv`; `Ok => mapped.contains(page_base(phys_addr))`; `Err => !mapped.contains(page_base(phys_addr))`.
- Assessment: captures core success mapping fact (via `mapped` semantics) but misses explicit idempotence/TLB-effect guarantees callers cite. Failure arm is very strong and may overstate behavior relative to "not newly installed" semantics.

### `identity_map_page`
- **Requires:** `identity_map_view().inv()`.
- **Ensures:** always `inv`; `Ok => accessible(phys_addr@)`; `Err => !accessible(phys_addr@)`.
- Assessment: captures high-level success accessibility, but misses several caller-required details (idempotence, explicit pre-init success/no-op law, error taxonomy, allocator non-consumption). Failure arm is strong and may exceed what callers truly need/safely know.

Overall spec quality verdict: **Not complete for caller use**; success/failure contracts are partially meaningful but incomplete, and some failure guarantees appear over-strong.

## Caller Coverage (Covered N/Total + missing)

Using `caller_analysis.md` expectations (success+failure bullets for the 3 in-scope functions):
- **Covered: 3 / 16 (strict full-coverage count).**

Covered examples:
1. `identity_map_page` on `Ok` gives accessibility (`accessible`).
2. `ensure_pte` on `Ok` gives `mapped.contains(page_base(..))`.
3. `identity_map_page`/`ensure_pte` provide explicit failure-state accessibility/membership predicates.

Missing (or only partial) expectations include:
- `identity_map_page`: idempotence/no-op safety, explicit pre-init-success/no-op law, writable+supervisor detail at contract boundary, TLB consistency statement, no-frame-allocator-consumption guarantee, error-code semantics.
- `ensure_pt`: PDE-present postcondition, returned PT tied to PDE, idempotent fast-path behavior, explicit failure framing expected by callers.
- `ensure_pte`: explicit idempotence/no-extra-invlpg behavior, explicit fresh-install TLB guarantee, error-code semantics.

## Proof Completeness (admit + external_body counts/locations)

Command run: `grep -rnE "admit|assume|external_body|assume_specification"` on the 3 module files, then code-token filtering.

- `admit()`: **0**
- `assume()`: **0**
- `external_body` attributes: **4**
  - `src/kernel/src/mm/virt/identity_map.rs:509` `#[verus_verify(external_body)]` (`ensure_pt`)
  - `src/kernel/src/mm/virt/identity_map.rs:607` `#[verus_verify(external_body)]` (`ensure_pte`)
  - `src/kernel/src/mm/virt/identity_map.rs:693` `#[verus_verify(external_body)]` (`identity_map_page`)
  - `src/kernel/src/mm/virt/identity_map.spec.rs:143` `#[verifier::external_body]` (`ExPageTableBss`)
- `assume_specification`: **1**
  - `src/kernel/src/mm/virt/identity_map.spec.rs:185` (bump allocator constructor)

Hard blocker check (per stated rules): `admit()`/`assume()` counts are zero.

## TCB Compliance (each external_body YES/NO)

Cross-checked each module `external_body` against `verus-ai-logs/tcb-allowed.md`:
- `identity_map.rs::ensure_pt` — **YES**
- `identity_map.rs::ensure_pte` — **YES**
- `identity_map.rs::identity_map_page` — **YES**
- `identity_map.spec.rs::ExPageTableBss` — **YES**

### Critical TCB legitimacy finding (targets-in-TCB)
I checked git history/diff versus base branch (`origin/dev`):
- `verus-ai-logs/tcb-allowed.md` is **absent** on `origin/dev` and appears as a **new file** on this effort branch.
- Therefore the listing that places the 3 **verification targets themselves** into TCB was introduced during this effort, not pre-fixed from base.

Given the hard rule "TCB fixed in advance; no new trust boundaries may be introduced", this is a **process-level blocker**. Also, with all three target functions marked `external_body`, no in-body proof was achieved for the target functions.

## Guardrails Compliance (exact counts)

Module-only exact counts (3 files):
- `admit()`: **0**
- `assume()`: **0**
- `external_body`: **4**
- `assume_specification`: **1**
- cfg-gated exec items: **1** (`#[cfg(feature = "test")]` test module)
- `#[cfg(verus_keep_ghost)] include!(...)`: **2** (ghost includes; not counted as cfg-gated exec)

## AST Consistency (PASS/FAIL)

Searched for `// VERUS REWRITE` in module files: **no matches**.
- **PASS** (no rewrite markers requiring semantic-equivalence adjudication).

## Verification (PASS/FAIL + summary)

Command run: `make verify-kernel MODULE=mm::virt`
- Exit code: **0**
- Verifier summary:
  - `verification: cached (no recompilation), — (exit 0)`
  - `status: CHEATING_DETECTED`
  - global cheating counts: `assume=0 external_body=23 admit=4 trusted=0 cfg_gate=19`
- Module share (from grep/module output):
  - `external_body=4`, `admit=0`, `assume=0`, `assume_specification=1` in target module files.

Additional required checks:
- `spec_drift.py ... identity_map.rs --before HEAD`: **No contract drift detected**.
- `fn_coverage.py --help`: ran successfully.
- `fn_coverage.py src/.../identity_map.rs src/.../identity_map.rs`: 14/14 exec functions matched (file-level presence check). In-scope contract annotations exist on all 3 target fns, but all 3 are `external_body`.

## Bug Summary

`bugs.md` says "None." For code logic defects, none were established here. However, for final verification status this is incomplete/misleading:
- The three in-scope target functions remain unproven in-body (`external_body`).
- They were added to a newly introduced TCB list during this effort.

So "no code bug" may be true, but there is a **verification-integrity issue** (proof deferral/trust escalation) that must be reported as unresolved.

## Issues (priority order)

1. **P0 — TCB policy violation risk:** target functions moved into newly introduced TCB during this effort (base branch lacked `tcb-allowed.md`), conflicting with "fixed in advance/no new trust boundary" rule.
2. **P0 — Core target functions unproven in-body:** `identity_map_page`, `ensure_pt`, `ensure_pte` are all `external_body`, so target proof objective is unmet.
3. **P1 — Contract incompleteness vs caller expectations:** multiple caller-required success/failure properties absent or only partial in contracts.
4. **P2 — Documentation inconsistency:** `bugs.md` "None" does not reflect the surviving verification/trust-gap status.

## Result: **FAIL**

**Reasoning:** Although `admit()`/`assume()` are zero and listed `external_body` items are documented, the in-scope target functions themselves are trusted (`external_body`) rather than proven, and that trust boundary was introduced via a new TCB file on this effort branch (not fixed beforehand in base). Under the stated hard rules, this is not an acceptable final verification outcome.
