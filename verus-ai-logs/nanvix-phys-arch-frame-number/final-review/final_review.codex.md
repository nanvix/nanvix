# Final Comprehensive Review — `arch-frame-number`

## Spec Quality

### Scope checked
- Type: `FrameNumber` (View + `inv`)
- Fns: `FrameNumber::from_raw_value`, `FrameNumber::into_raw_value`

### Contract assessment
- `from_raw_value` contract is precise and complete:
  - `value <= spec_max` => `Some` with exact value preservation (`result->Some_0@ == value as int`)
  - `value > spec_max` => `None`
  - Success/failure partition is total and non-tautological.
- `into_raw_value` contract is correct:
  - exact projection (`result as int == self@`)
  - in-range bound (`0 <= self@ <= spec_max`) needed by callers for overflow-safe `<< FRAME_SHIFT` reasoning.
- View/invariant quality:
  - `View` is minimal and caller-facing (`int` frame index).
  - `inv` captures the required semantic bound exactly (`0 <= self@ <= spec_max`).

### `spec_max` design divergence vs `view_design.md`
- `view_design.md` proposes `uninterp spec_max` + `assume_specification[FrameNumber::MAX]`.
- Shipped code uses:
  - `open spec fn spec_max() -> nat { (MAX_ADDRESS / FRAME_SIZE - 1) as nat }`
  - no `assume_specification`.
- Judgment: shipped approach is **stronger and preferable** (less trust, direct definitional linkage).

### Concrete correctness checks
- Exec constant: `FrameNumber::MAX = MAX_ADDRESS / FRAME_SIZE - 1`.
- Spec bound: `spec_max()` uses the same arithmetic expression.
- Constants in arch:
  - `MAX_ADDRESS = usize::MAX`
  - `FRAME_SIZE = PAGE_SIZE = 4096`
- Therefore `MAX_ADDRESS / FRAME_SIZE >= 1` (on 32/64-bit `usize`), so `- 1` does not underflow; cast to `nat` is sound.
- Boundary match is exact: body checks `value > MAX`; spec splits on `value as int > spec_max()` / `<= spec_max()`; since formulas are identical, behavior matches exactly.

### Type-invariant discharge on constructor path
On `Some(Self(value))` path, branch condition guarantees `value <= MAX`; with `value: usize`, `0 <= value as int`; and `MAX == spec_max() as int`. So `inv()` (`0 <= self@ <= spec_max`) is satisfied for constructed value.

### Note on `NULL`
`NULL` has `ensures Self::NULL@ == 0`; this is consistent and harmless (associated-const spec, not a concern for in-scope function soundness).

## Caller Coverage

Covered **4/4** expected properties.

1. Round-trip identity — **Covered** by:
   - `from_raw_value`: success preserves abstract value
   - `into_raw_value`: returns exact abstract value
2. Out-of-range rejection — **Covered** by:
   - `from_raw_value`: `value > spec_max() ==> None`
3. In-range bound for overflow-safe shift — **Covered** by:
   - type invariant `inv`
   - `into_raw_value` ensures boundedness
4. Totality of `into_raw_value` — **Covered**:
   - no `requires`, returns `usize` directly.

Missing list: **None**.

## Proof Completeness

In module files (`number.rs`, `number.spec.rs`, `number.proof.rs`):
- `admit`: **0**
- `external_body`: **0**

Any `admit` would be BLOCKER; none present.

## TCB Compliance

Arch-wide `external_body` entries provided by orchestrator:
1. `x86/mem/paging/mod.rs::invlpg`
2. `x86/mem/paging/table.rs::Table::<E>::read`
3. `x86/mem/paging/table.rs::Table::<E>::write`

All three are explicitly listed in `verus-ai-logs/tcb-allowed.md`.

## Guardrails Compliance

### In-scope module exact counts
- `admit`: **0**
- `assume`: **0**
- `external_body`: **0**
- `assume_specification`: **0**
- cfg-gated exec: **0**
  - (`#[cfg(verus_keep_ghost)]` only used for standard `include!` of spec/proof files)

### Arch-wide authoritative counts (provided)
- `assume=0`, `admit=0`, `trusted=0`, `no_decreases=0`, `cfg_gate=0`, `external_body=3`.

BLOCKER thresholds (`admit>0` or `assume>0`) are not triggered.

## AST Consistency

**PASS**.
- No `// VERUS REWRITE` comments in `src/libs/arch/src/x86/mem/paging/frame/`.
- `ast_consistency.py` check on `number.rs`: `✅ Consistent: 4 functions, 1 structs match.`
- No semantic exec/spec mismatch found for in-scope items.

## Verification

**PASS** (per authoritative orchestrator data, as required):
- `make verify-arch`: exit 0
- spec drift: 0
- function coverage: 4/4 exec fns matched; both in-scope public fns specced.

## Bug Summary

- `bugs.md` at target path is missing (treated as zero recorded bugs).
- Independent review found **no real defect** in in-scope functions/type contracts.
- `into_raw_value`’s `proof! { use_type_invariant(self); }` is appropriate and correctly used to surface `inv` facts for postconditions/callers.
- No `external_body` exists in this module that could mask a defect.

## Issues (highest priority first)

1. **P3 / Informational** — `view_design.md` is stale relative to shipped implementation (`uninterp+assume_specification` proposed, but concrete `spec_max` was implemented). This is not a soundness issue; shipped code is stricter and better.

## Result

**PASS**.

Justification: zero blockers; all required dimensions satisfied (spec quality, caller coverage 4/4, proof completeness, TCB compliance, guardrails, AST consistency, verification status, and bug reconciliation).
