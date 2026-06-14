# Final Independent Review — `mm::phys::kframe`

## Spec Quality

- `KernelFrame::new` has meaningful success contract (`Ok(kf) => kf@ == base@ && kf.inv()`) and proper precondition (`base.inv()`).
- `KernelFrame::new` error contract is tautological (`Err(_) => true`) and does **not** encode caller-relied non-consumption/non-free behavior. **BLOCKER**.
- `KernelFrame::base` contract is strong and usable: `requires self.inv(); ensures result@ == self@; ensures result.inv();`.
- `KernelFrame::inv` is non-trivial (`self@ % spec_page_size() == 0`), so not vacuous.
- `KernelFrame::drop` has `opens_invariants none` + `no_unwind` but no abstract postcondition; this does not capture caller expectation “frees exactly once”. **BLOCKER**.

## Caller Coverage

Covered **4/6** expectations.

1. `new` requires valid/page-aligned input (`base.inv()`) — **Covered** (`kframe.rs:84`).
2. `new` success preserves address (`kf@ == base@`) — **Covered** (`kframe.rs:88`).
3. `new` failure does not consume/free frame — **Missing** (`Err(_) => true`, `kframe.rs:91`).
4. `base` returns exact owned address (`result@ == self@`) — **Covered** (`kframe.rs:132`).
5. `base` returns aligned `FrameAddress` for downstream conversions — **Covered** (`kframe.rs:133` + `kframe.spec.rs:11-13`).
6. `drop` frees underlying frame exactly once — **Missing** (no postcondition on allocator state; `kframe.rs:193-201`).

Missing list:
- `new` error-path non-consumption guarantee.
- `drop` deallocation-effect guarantee (“free exactly once”).

## Proof Completeness (kframe-only)

- `admit()` count: **0** (no matches in `kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`).
- `external_body` count: **1**
  - `src/kernel/src/mm/phys/kframe.rs:94` — `KernelFrame::new`.

No kframe-local `admit` blockers.

## TCB Compliance

**YES**.

The only kframe `external_body` (`KernelFrame::new`) is explicitly allowlisted in `verus-ai-logs/tcb-allowed.md` (lines 16-25, and repeated in cross-module section).

## Guardrails (kframe-only exact counts)

- `admit`: **0**
- `assume`: **0**
- `external_body`: **1** (`kframe.rs:81/94`, `KernelFrame::new`)
- `assume_specification`: **0**
- cfg-gated exec: **1** (`kframe.rs:199`, `#[cfg(not(verus_keep_ghost))] error!(...)` in `drop`)

Blocker criteria check:
- `admit > 0`: no
- `assume > 0`: no
- unallowlisted `external_body`: no

## AST Consistency

**PASS**.

- `// VERUS REWRITE` occurrences in kframe files: **0** (none to reconcile).
- `drop` cfg-gate analysis: `#[cfg(not(verus_keep_ghost))]` guards only logging (`error!`) inside the already-taken `Err` branch of `frame::free`. It does not gate allocation/free calls, ownership transfer, branching, return values, or state transitions. This is logging-only, not control-flow cheating.

## Verification

**PASS (static confirmation only, as requested).**

Confirmed from `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`:
- kframe footprint is exactly `mm/phys/kframe.rs:94 new: external_body`.

Confirmed by grep over kframe files:
- `admit()` absent.
- `assume` / `assume_specification` absent.
- one cfg-gated exec site in `drop` logging.

Did **not** rerun `make verify-kernel` (per instruction).

## Bug Summary

From `verus-ai-logs/nanvix-phys-phys-kframe/bugs.md`:

1. Duplicate `vstd::prelude::*` import (build-hygiene) — **Fixed**; not a runtime logic defect.
2. “`KernelFrame::new` retains `external_body`” entry — correctly categorized as a verification trust-boundary note (not a code correctness bug).

Unrecorded real code defects discovered: **none**.

Unrecorded verification/spec defects discovered:
- Missing `new` error-path non-consumption guarantee.
- Missing `drop` deallocation-effect guarantee.

## Issues (priority order)

1. **BLOCKER** — `KernelFrame::new` error arm is vacuous (`Err(_) => true`), so caller-required non-consumption/non-free behavior is unproved at API level.
   - Suggested fix: strengthen `Err` postcondition to preserve allocator state / non-consumption fact (or equivalent ownership predicate).
2. **BLOCKER** — `KernelFrame::drop` has no abstract postcondition, so “frees exactly once” caller expectation is not captured in spec.
   - Suggested fix: add allocator-view postcondition for free effect (or explicitly document/accept this as trusted gap in the verification target).

## Result

**FAIL** — 2 blockers (spec/caller-contract completeness), despite clean kframe-local cheating profile (0 admit/assume, 1 allowlisted external_body).
