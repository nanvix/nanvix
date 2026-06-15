# Final Review — `sys-virt-address`

## Checklist

### Caller Analysis
- [x] Read `caller_analysis.md` and extracted in-scope caller expectations.
- [ ] All caller-required properties are covered by in-scope formal contracts.

### View Design
- [x] Read `view_design.md` and compared against current `View` implementation.
- [x] `VirtualAddress` view exists and is abstraction-level (`type V = int`, `view() = self.0 as int`).
- [ ] Round-trip/inverse property required by callers is specified on `into_raw_value`.

### Specification
- [x] `VirtualAddress::new` has `#[verus_spec]` with `ensures result@ == value as int`.
- [x] Inherent `VirtualAddress::from_raw_value` has `#[verus_spec]` with `ensures result@ == raw_addr as int`.
- [ ] Trait `Address::from_raw_value` impl has verified external-top contract.
- [ ] `Address::into_raw_value` has `#[verus_spec]` (`result as int == self@`) as required by callers.
- [ ] `impl Address for VirtualAddress` is under `#[verus_verify]`.

### Proving
- [x] No `admit()` in `virt.rs` / `virt.spec.rs` / `virt.proof.rs`.
- [x] No `#[verifier::external_body]` in `virt.rs` / `virt.spec.rs` / `virt.proof.rs`.

### Cheating Elimination
- [x] `admit/assume/external_body/assume_specification` all absent in module files.
- [x] `spec_drift.py` reports no contract weakening.
- [ ] `make verify-sys` overall status is clean (it reports `status: CHEATING_DETECTED`).

### Bug Recording
- [x] Checked `bugs.md` presence.
- [x] Reconciled recorded bugs (none recorded).
- [x] Identified new review issues and classified severity in this report.

## Spec Quality

`new` and inherent `from_raw_value` have concise, caller-usable postconditions and match the view design (`int` abstraction).

Major gaps for in-scope API:
1. `Address::into_raw_value` has **no** `#[verus_spec]` ensures; required caller fact `addr.into_raw_value() == addr@` is not available.
2. `impl Address for VirtualAddress` lacks `#[verus_verify]`, so trait-method contracts are not in the verified surface; this weakens confidence in trait-level API obligations.
3. Trait `from_raw_value(usize) -> Result<Self, Error>` has no formal contract despite being in requested review set.

## Caller Coverage

Coverage basis: caller-facing key invariants in `caller_analysis.md` (5 total).

**Covered: 2 / 5**
- Covered:
  - Constructor equivalence at view level (`new` and inherent `from_raw_value` both map input to `result@`).
  - View abstraction (`VirtualAddress@` is `int`, via `impl View`).
- Missing / insufficiently specified:
  1. Round-trip identity (`new(x).into_raw_value() == x`, `from_raw_value(x).into_raw_value() == x`) — missing `into_raw_value` spec.
  2. Trait-level constructor/projection obligations (`Address` impl methods) are not verified/spec’d.
  3. Purity/inverse guarantee for `into_raw_value` (no masking/offset/loss) is not formalized.

## Proof Completeness

- `admit()` list: **None** (count = 0).
- `external_body` not in TCB list: **None** (module `external_body` count = 0).

## TCB Compliance (YES/NO)

**YES** — no `external_body` appears in `virt.rs`, `virt.spec.rs`, or `virt.proof.rs`; therefore no non-TCB trust introduced in scope.

## Guardrails Compliance

Scanned files: `virt.rs`, `virt.spec.rs`, `virt.proof.rs`.

- `admit`: **0**
- `assume`: **0**
- `external_body`: **0**
- `assume_specification`: **0**
- `cfg-gated exec` (cheating form `cfg(not(verus_keep_ghost))`): **0**

Notes:
- `virt.rs` has `#[cfg(verus_keep_ghost)] include!(...)` at lines 9/11 (ghost includes), and target-width cfgs (lines 39/296); no `cfg(not(verus_keep_ghost))` exec-duplication pattern found.

## AST Consistency (PASS/FAIL)

**FAIL** (strict policy)

Evidence:
- `ast_consistency.py --base-ref verus-ai/sys-virt-address ... count` reported:
  - `⚠️  3 mismatched (16 functions match)`
- Detailed report command:
  - `report -o .../ast-report-sysvirt` summary says `Functions mismatched: 0`, `Consistent: YES`.

No `// VERUS REWRITE` comments were found in in-scope files.

Given the explicit rule “any MISMATCH is a blocker”, the count/summarized mismatch signal is treated as blocking despite contradictory detailed output.

## Verification (verus PASS/FAIL, build PASS/FAIL)

- `make verify-sys`: **FAIL** (strict)
  - Command exit: `verify_sys_exit=0`
  - Reported by toolchain summary:
    - `Exit code : 0`
    - `cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=1`
    - `status: CHEATING_DETECTED`
  - Error-line count in captured output: `verify_sys_error_lines=0`

- `make build`: **PASS**
  - `build_exit=0`
  - Output tail: `make: Nothing to be done for 'build'.`
  - Error-line count: `build_error_lines=0`

## Bug Summary

- Total recorded in `bugs.md`: **0** (`bugs.md` not present).
- True bugs found in this review: **None (runtime/functional)**.
- Verification/spec defects newly identified (not recorded in `bugs.md`):
  1. Missing formal contract on `into_raw_value` (severity: **high**, proof-coverage/soundness gap for callers).
  2. Unverified trait impl surface (`impl Address for VirtualAddress` lacks `#[verus_verify]`) (severity: **high**).

## Issues (highest priority first)

1. **BLOCKER** — Missing spec on in-scope `Address::into_raw_value`.
   - Callers rely on `addr.into_raw_value() == addr@`; no formal ensures currently provides this.
2. **BLOCKER** — `impl Address for VirtualAddress` is not `#[verus_verify]`.
   - Trait-level in-scope API obligations are not part of verified contract surface.
3. **BLOCKER (strict-rule)** — AST count check reports mismatches.
   - Even with contradictory detailed report, strict rule treats mismatch signal as blocking.
4. **MAJOR** — Caller coverage incomplete (2/5 key invariants formally covered).
5. **MAJOR** — `make verify-sys` reports `CHEATING_DETECTED` (`cfg_gate=1`), so verification status is not clean.

## Result: FAIL
