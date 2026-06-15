# Final Independent Review — `sys-virt-address` (`VirtualAddress`)

## Checklist
- [x] 1) Spec quality reviewed (contracts + View/inv design, closed/open choice, math types)
- [ ] 2) Caller coverage complete for all listed expectations
- [x] 3) Proof completeness checked (`admit`, `external_body`)
- [x] 4) TCB compliance checked (no new in-module trust boundary)
- [x] 5) AST consistency run; `VERUS REWRITE` comments checked
- [x] 6) Verification/build run (`make verify-sys`, `make build`)
- [x] 7) Guardrails counted across `virt.rs` / `virt.spec.rs` / `virt.proof.rs`
- [x] 8) Bugs reconciliation performed (`bugs.md` vs code/logs)
- [x] 9) Spec drift checked (`spec_drift.py` + manual diff reconciliation)

## Spec Quality
In-scope specs found:
- `VirtualAddress::new` (`virt.rs:53-57`): `result@ == value as int`, `result.inv()`
- Inherent `VirtualAddress::from_raw_value` (`virt.rs:71-75`): `result@ == raw_addr as int`, `result.inv()`
- `View` (`virt.rs:333-339`): `type V = int`, `closed spec fn view`
- `inv` (`virt.spec.rs:13-15`): `0 <= self@ <= usize::MAX as int` (`open`)

Assessment:
- Correct and understandable for a thin newtype identity abstraction.
- `closed view` and `open inv` choices are appropriate.
- No missing error-path ensures for in-scope infallible APIs.
- Minor redundancy: `result.inv()` is logically implied by identity + `usize` input (subsumed, not unsound).

## Caller Coverage
**Covered 4 / 6 expectations. Missing 2.**

Covered:
1. `new` identity (`result@ == value as int`) — covered.
2. Inherent `from_raw_value` identity — covered.
3. Round-trip `new(a).into_raw_value()==a` — available via constructor specs + consumer-side `assume_specification` in `phys.spec.rs:107-112`.
4. Round-trip `from_raw_value(a).into_raw_value()==a` — same.

Missing / uncovered:
1. **`into_raw_value` purity contract** not directly specified in-module (method at `virt.rs:269-271` has no `#[verus_spec]`; only consumer-side trust boundary exists).
2. **Ord/Eq agreement with raw integer** not explicitly specified/lemma-backed for callers (relied upon per `caller_analysis.md`).

## Proof Completeness
Across in-scope files (`virt.rs`, `virt.spec.rs`, `virt.proof.rs`):
- `admit(...)`: **0**
- `#[verifier::external_body]`: **0**

`virt.proof.rs` is empty (`verus! { }`).
No blocker from `admit`/`external_body` counts.

## TCB Compliance
- No new `external_body` or `assume_specification` introduced inside the reviewed module files.
- The `<VirtualAddress as Address>::into_raw_value` trust boundary is consumer-side (`src/kernel/src/hal/mem/types/address/phys.spec.rs:107-112`) and is allow-listed in `verus-ai-logs/tcb-allowed.md:263-267`.

Verdict: compliant with stated TCB placement.

## Guardrails Compliance (exact counts)
Scope: only `virt.rs`, `virt.spec.rs`, `virt.proof.rs`.

| Dimension | Count | Locations |
|---|---:|---|
| `admit` | 0 | none |
| `assume(...)` | 0 | none |
| `external_body` | 0 | none |
| `assume_specification` (code) | 0 | none |
| cfg-gated exec cheating (`cfg(not(verus_keep_ghost))` on exec bodies) | 0 | none |

Legitimate cfg usage (non-cheating):
- `virt.rs:9,11` `#[cfg(verus_keep_ghost)]` includes of spec/proof files
- `virt.rs:39,308` `#[cfg(target_pointer_width = "32")]`

Note: string `assume_specification` appears once in a **comment** (`virt.rs:266`), not as code.

## AST Consistency
Commands/results:
```text
python3 .../ast_consistency.py src/libs/sys/src/sys/mm/address/virt.rs count
✅ Consistent: 18 functions, 1 structs match.
```
`VERUS REWRITE` occurrences in scope files: **0**.

Note: `--base-ref verus-ai/hal-platform-microvm` mode reported name-collision mismatches for overloaded methods, but auto-detect mode (recommended on `verus-ai/*` branches) reported full consistency.

## Verification
```text
make verify-sys
Exit code : 0
Cheating Pattern Check: ✅ No cheating detected.

make build
make: Nothing to be done for 'build'.
[exit:0]
```
- `make verify-sys`: **PASS**, exact verification errors: **0**
- `make build`: **PASS**, build errors: **0**

## Bug Summary
`bugs.md` says **None**. Reconciliation result:
- No concrete code bug found in in-scope identity constructors/projection.
- The `<VirtualAddress as Address>::into_raw_value` non-body-verified status is documented as a Verus front-end/trait-impl limitation in `verus-unsupported.md` and mapped to an allow-listed consumer-side trust boundary.
- Classification as limitation (not code bug) is reasonable given current evidence/logging.

## Issues (priority order)
1. **P1 — Missing explicit caller-facing coverage for Ord/Eq agreement and into_raw purity**
   - Caller expectations include these properties, but current in-module specs do not explicitly provide them.
2. **P3 — Redundant postconditions (`result.inv()`)** on infallible identity constructors (subsumed by stronger identity fact + type range).

## Result: **FAIL**
Reason: checklist item (2) is not fully satisfied (caller expectation coverage incomplete: 2 missing items).
