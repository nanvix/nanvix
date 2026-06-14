# Final Independent Review — `hal::mem::types::address::frame` (`FrameAddress`)

## Checklist
- **Caller Analysis:** **FAIL** — 11/12 caller expectations are contract-covered; `from_raw_value` error semantics remain under-specified (`Err(_) => true`).
- **View Design:** **PASS** — `FrameAddress` view (`int`) + `inv()` (page alignment) are minimal, caller-facing, and non-leaky.
- **Specification:** **FAIL** — one tautological error arm exists (`from_raw_value`), leaving failure/liveness behavior unconstrained.
- **Proving:** **PASS** — no `admit()`, no `external_body` in-scope; proof lemmas are concrete and used.
- **Cheating Elimination:** **FAIL** — no `admit/assume/external_body`, but 1 workspace-internal `assume_specification` remains (trusted placeholder).
- **Bug Recording:** **PASS** — BUG-001 (duplicate `use ::vstd::prelude::*;`) is fixed; one additional undocumented spec-quality issue identified.

## Spec Quality
**FAIL**

Findings:
1. `FrameAddress::from_raw_value` uses `Err(_) => true` (tautological error arm). This is explicitly weak per spec-design guidance.
2. Success arms are otherwise meaningful and non-vacuous:
   - `from_raw_value`: `Ok(fa) => fa.inv() && fa@ == raw_addr as int`
   - `into_raw_value`: `result as int == self@`
   - `from_frame_number`: total-success + alignment + exact address mapping
   - `into_frame_number`: requires alignment+representability; ensures exact frame index and reconstruction of `self@`
3. Conversion helpers are grounded, not vacuous:
   - `spec_page_size() = ::arch::mem::PAGE_SIZE as int` (arch constant is concrete/verified)
   - `spec_frame_number(addr)=addr/spec_page_size()`
   - `spec_from_number(frame)=frame*spec_page_size()`
   - `spec_frame_raw_value(frame)=frame@`
   - `spec_max_frame_number()=FrameNumber::spec_max() as int`, with concrete `FrameNumber::spec_max()` body.

## Caller Coverage
**Covered 11 / 12**

Covered expectations:
- `from_frame_number`: canonical mapping to frame base address, alignment, no observable `Err` path usage impact.
- `into_frame_number`: exact frame-index extraction and address reconstruction.
- `from_raw_value` success semantics (`fa@ == raw_addr`, aligned).
- `into_raw_value` physical-address projection (`result as int == self@`) and composition with frame-number specs.
- Type-level abstraction (`FrameAddress` as page-aligned physical frame handle) is preserved.

Missing / weakly specified:
1. **`from_raw_value` failure semantics** — no bidirectional failure condition or liveness guarantee; spec permits arbitrary `Err` even when success is possible.

## Proof Completeness
- **admit:** `0` (locations: none)
- **external_body-not-in-tcb:** `0` (list: none)

## TCB Compliance
**YES (external_body), with caveat**

- In-scope `external_body`: none.
- No new trust boundary introduced.
- Caveat: one `assume_specification` remains in `frame.spec.rs` for workspace-internal `PhysicalAddress::from_raw_value` (recorded in `tcb-allowed.md` lines 154–168).

## Guardrails Compliance
- `admit`: **0**
- `assume`: **0**
- `external_body`: **0**
- `assume_specification`: **1**
- `cfg-gated exec`: **0** (`#[cfg(verus_keep_ghost)]` appears only on spec/proof includes, not exec behavior)

Verdict on the single `assume_specification` placeholder:
- **FAIL (strict policy)** for final sign-off under “no workspace-internal assume_specification”.
- It is documented and pre-recorded, but still a trusted intra-workspace axiom.

## AST Consistency
**PASS** — `ast_consistency.py --base-ref 38885545d~1 frame.rs count` reports consistent (9 functions, 1 struct; no mismatches).

## Verification
**PASS** — central verification artifacts report:
- module verify: pass, no cheating detected in `hal::mem::types::address::frame`
- full `make verify`: pass
- `./z build -- all-kernel`: pass

## Bug Summary
- **Total:** 2
- **True Bugs:** 0
- **Context-Dependent / Spec Bugs:** 1 (medium correctness)
- **Fixed Historical Bug:** 1 (BUG-001 duplicate `vstd` import, fixed)

New undocumented issue:
1. **Context-Dependent (spec-quality):** `from_raw_value` has tautological error arm (`Err(_) => true`), weakening caller-provable behavior on failure/liveness.

## Issues (highest priority first)
1. **BLOCKER:** Workspace-internal `assume_specification` remains (`<PhysicalAddress as Address>::from_raw_value`) — documented but still trusted intra-workspace boundary.
2. **BLOCKER:** `from_raw_value` error path is tautological (`Err(_) => true`), leaving failure condition/liveness underspecified.
3. **Minor:** Some ensures are stronger-than-needed duplicates for usability; not unsound, but increases spec noise.

## Result: **FAIL**

Reason: strict review requires all dimensions to pass; specification/coverage quality and strict guardrail policy on workspace-internal `assume_specification` do not fully pass.
