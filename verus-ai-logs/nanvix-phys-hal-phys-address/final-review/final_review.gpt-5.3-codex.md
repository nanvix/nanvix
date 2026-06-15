# Final Independent Review — `hal::mem::types::address::phys`

## Scope Checked
- Type: `PhysicalAddress` (`View` + `inv`)
- Functions: `from_number`, `into_frame_number`, `from_mmio_address`
- Files read: `phys.rs`, `phys.spec.rs`, `phys.proof.rs`, caller/view docs, TCB allowlist, shared evidence, 4 skill docs.

## Per-item Checklist Verdicts
| Item | Verdict | Notes |
|---|---|---|
| Caller Analysis | PASS | Expectations mapped to contracts/invariant (see coverage). |
| View Design | PASS | `View=int`, `inv` = representable frame index; matches caller-facing abstraction. |
| Specification | PASS | In-scope contracts are present and load-bearing. |
| Proving | PASS | No `admit()`; proof obligations discharged in-body (`proof!` blocks). |
| Cheating Elimination | PASS | `assume=0`, `admit=0`; only approved trust edges used. |
| Bug Recording | PASS | `bugs.md` absent; independent defect check found no real bug. |

## Spec Quality (strict)
- `from_number`: ensures base-address relation (`frame*page_size`), alignment, and `inv`.
- `into_frame_number`: requires `self.inv()` and ensures exact frame projection.
- `from_mmio_address`: requires frame-representability; ensures `Ok`, identity wrap, and `inv`.
- `inv`: exactly the totality precondition for `into_frame_number` unwrap.
- `View`: `self@ == self.0@` (raw address abstraction).
- `assume_specification` edges (6) are minimal and load-bearing (frame size/shift identities; newtype/raw projections; frame number range/success criterion).

## Caller Coverage
**Covered: 11 / 11. Gaps: 0.**

1. `from_number` yields `frame*FRAME_SIZE` → covered by `result@ == spec_from_number(...)`.
2. `from_number` page-aligned → covered by `result@ % spec_page_size() == 0`.
3. `from_number` establishes invariant → covered by `result.inv()`.
4. `into_frame_number` total/no panic → covered by `requires self.inv()` + range proof to `from_raw_value(...).unwrap()`.
5. `into_frame_number == addr >> FRAME_SHIFT` → body + `lemma_usize_shr_is_div`; ensures ties result to `self@/FRAME_SIZE`.
6. `into_frame_number == addr / FRAME_SIZE` → covered by `spec_frame_raw_value(result) == spec_frame_number(self@)`.
7. `into_frame_number` in-range index → implied by `inv` + `FrameNumber::from_raw_value` spec.
8. `from_mmio` identity wrap → covered by `result matches Ok(r) ==> r@ == addr@`.
9. `from_mmio` returns `Ok` (in contract domain) → covered by `result is Ok`.
10. `from_mmio` bypasses RAM validator → covered (no RAM-validity precondition/ensures; exec body is direct wrap).
11. Type invariant guarantees representable frame number for unwrap totality → covered by `inv` definition and `into_frame_number` contract.

## Proof Completeness
Module-local counts (`phys.rs`, `phys.spec.rs`, `phys.proof.rs`):
- `admit(...)`: **0**
- `assume(...)`: **0**
- `#[verifier::external_body]`: **1** (`phys.spec.rs:39`, `ExFrameNumber`)
- `#[verifier::external_type_specification]`: **1** (`phys.spec.rs:38`, `ExFrameNumber`)
- `assume_specification[...]`: **6** (`phys.spec.rs:79,88,96,103,112,119`)
- `#[cfg(verus_keep_ghost)]` in `phys.rs`: **3** (`include!`, `include!`, `use`) 
- cfg-gated exec branches/expressions/match arms in `phys.rs`: **0**

## TCB Compliance
- In-scope `external_body` is only `ExFrameNumber`.
- It is explicitly allowlisted in `verus-ai-logs/tcb-allowed.md` under:
  - “Allowed `external_type_specification` — `phys.spec.rs::ExFrameNumber`”.
- No new trust boundary introduced in this module.

## AST Consistency (independent re-derivation)
**PASS** (with pre-approved deviations only).

I independently diffed `phys.rs` against `origin/dev` and re-derived both flagged rewrites:
1. `from_number`: `frame.into_raw_value() * FRAME_SIZE` rewritten as `let frame_raw; let page_size; frame_raw * page_size` + proof block. Semantics preserved; this is exactly pre-approved `f(complex_expr) -> let x=complex_expr; f(x)`.
2. `into_frame_number`: `raw_addr >> FRAME_SHIFT` rewritten as `let shift = FRAME_SHIFT; raw_addr >> shift` + proof block. Semantics preserved; same pre-approved deviation class.

`// VERUS DEVIATION` comments are present at both sites (`phys.rs` around lines 156 and 199).

## Verification
**PASS**.

Independent run:
- Command: `make verify-kernel MODULE=hal::mem::types::address::phys`
- Exit: `0`
- Output: `No cheating detected in module hal::mem::types::address::phys`.
- Log: `verus-ai-logs/verify-kernel/verus-logs/verus_2026-06-15_13-20-12.log`

## Guardrails
- `admit > 0` blocker? **No** (`0`)
- `assume > 0` blocker? **No** (`0`)
- external_body outside allowlist blocker? **No** (only allowlisted `ExFrameNumber`)
- cfg-gated exec in `phys.rs` blocker? **No** (gates only `include!`/`use`)

## Bug Summary
- `bugs.md` does not exist; independent in-scope defect check found no true bug.
- Checked specifically for: multiply overflow (`from_number`), unwrap panic (`into_frame_number`), frame arithmetic mismatch (`>>` vs `/`), alignment error, and MMIO identity mismatch.
- Classification per bug-reporting skill: **None**.

## Issues (priority order)
1. **None.**

## Result
# PASS
All required checklist items pass; no blockers found.
