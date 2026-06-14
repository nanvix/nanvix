# Final Independent Review — bump-allocator Verus effort

## Spec Quality

Reviewed files: `src/libs/bump_allocator/src/lib.rs`, `lib.spec.rs`, `lib.proof.rs`.

- **`alloc` / `alloc_as` external-top contracts are under-specified on failure paths.**
  - `alloc`: `Err(_) => true` (lib.rs:283) is tautological and does not encode caller-required failure behavior.
  - `alloc_as`: `Err(_) => true` (lib.rs:364) is tautological for propagated allocator errors.
  - Given `caller_analysis.md`, these are **not acceptable** as final API contracts because they omit no-consumption, exhaustion boundary, and error-specific guarantees.
- **Success-path guarantees are only partial.**
  - `alloc`/`alloc_as` do specify alignment + in-bounds for the returned slot.
  - They do **not** specify uniqueness/non-aliasing across allocations.
- **`slot_ref_addr` is uninterpreted (`lib.spec.rs:50`).**
  - Alignment/in-bounds clauses are meaningful only as abstract constraints over this uninterpreted address.
  - They do not connect to concrete pointer identities used by runtime callers/tests (e.g., pointer inequality), so practical caller utility is limited without stronger linking specs.
- **`assume_specification` on `<usize>::div_ceil`** (`lib.spec.rs:28`) is an external-bottom trust boundary.
  - vstd search performed: no `div_ceil` spec found under `/home/ruize/toolchain/verus/vstd`.
  - Rust docs checked (`~/.rustup/.../core/src/num/uint_macros.rs:3694-3718`): `div_ceil` rounds up and panics on zero divisor.
  - Current assumption (`requires y != 0`, quotient formula) is **acceptable** as a std boundary.
- **Spec drift check** (`spec_drift.py ... --before HEAD`): **PASS** (no drift).

## Caller Coverage (11/17 + missing list)

Coverage judged against caller expectations in `caller_analysis.md` (success + failure obligations).

Covered:
1. `align_up` success semantics via `align_up_spec` match.
2. `align_up` `None` semantics (alignment=0/overflow) via `align_up_spec`.
3. `alloc` success alignment.
4. `alloc` success in-bounds.
5. `alloc_as` success enforces `size_of::<T>() == N`.
6. `alloc_as` success enforces `align_of::<T>() <= A`.
7. `alloc_as` success alignment.
8. `alloc_as` success in-bounds.
9. `alloc_as` failure `SizeMismatch => size mismatch`.
10. `alloc_as` failure `AlignmentMismatch => alignment mismatch`.
11. `as_mut_ptr` abstract base stability (`result == base_of::<Self>()`).

Missing:
1. `alloc` uniqueness / non-aliasing / distinct-from-prior-slots guarantee.
2. `alloc` no-spurious-consumption-on-error guarantee.
3. `alloc` exhaustion boundary semantics (`Exhausted` iff capacity exhausted).
4. `alloc` propagated error semantics (`Overflow`/`OutOfBounds`/`Misaligned`) beyond tautological `Err(_)`.
5. `alloc_as` no-memory-handed-out / no-consumption guarantee for non-size/non-align failures (propagated `alloc` errors).
6. `as_mut_ptr` alignment and writable-range (`>= STORAGE_SIZE`) obligations are not encoded in requires/ensures.

## Proof Completeness (admit count+locations, external_body count+locations)

- `admit()` count across `lib.rs`, `lib.spec.rs`, `lib.proof.rs`: **0**
  - Locations: **none**
- `external_body` count across same files: **2**
  - `src/libs/bump_allocator/src/lib.rs:271` (`FixedSizeBumpAllocator::alloc`)
  - `src/libs/bump_allocator/src/lib.rs:348` (`FixedSizeBumpAllocator::alloc_as`)

## TCB Compliance

Checked against `verus-ai-logs/tcb-allowed.md`.

- `alloc` external_body: listed (allowed list lines 16-20).
- `alloc_as` external_body: listed (allowed list lines 21-23).
- External bodies not in allowed list: **none**.

## Guardrails Compliance

`admit: 0, assume: 0, external_body: 2, assume_specification: 1, cfg-gated exec: 0`

Notes:
- `cfg` usages found in `lib.rs` are only crate-level `no_std`, ghost include gates, and `#[cfg(test)]`; none are cfg-gated exec branches/expressions/match-arms.

## AST Consistency (PASS/FAIL)

Command run:
`python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/libs/bump_allocator/src/lib.rs`

Result: **PASS** (`12/12` functions matched, `0` mismatches).

`// VERUS REWRITE` comments grep result: **none found**. Semantic-equivalence check: **PASS (vacuous)**.

## Verification (PASS/FAIL + error count)

Command run: `make verify-bump-allocator`

Latest run result:
- Exit code: **0**
- Verus errors: **0** (`verification results:: 6 verified, 0 errors`)
- Script status: `CHEATING_DETECTED` due `external_body=2` (both are TCB-approved).

Section verdict: **PASS (0 errors)**.

## Bug Summary

Source reviewed: `verus-ai-logs/nanvix-phys-bump-allocator/bugs.md`.

- Existing entry content: “No code bugs found...” and explanatory claims.
- Current status:
  - No concrete runtime code bug proven in this review.
  - However, there is an **unrecorded spec-quality issue**: public contracts for `alloc`/`alloc_as` are under-specified on failure paths (`Err(_) => true`) and miss caller-required guarantees.
- Surviving unresolved **verification failures** to classify (True Bug / Context-Dependent / False Positive): **none** (verification run has 0 errors).
- `lib.proof.rs` header comment says bodies are `admit()` placeholders; this is **stale** (actual admit count is 0).

## Issues (priority ordered)

1. **P0 — External-top contract gap in `alloc`/`alloc_as` failure semantics**
   - Tautological `Err(_) => true` leaves critical caller expectations unspecified.
2. **P1 — Missing uniqueness/non-aliasing contract for successful allocations**
   - Caller analysis requires distinct non-overlapping slots.
3. **P1 — `as_mut_ptr` contract missing alignment + writable-range obligations**
   - Only base equality is specified.
4. **P2 — Stale proof-file header comment**
   - Mentions admit placeholders though none remain.

## Result: FAIL

FAIL rationale: not all caller expectations are covered by requires/ensures (11/17 covered). All other hard checks pass (admit=0, assume=0, all external_body TCB-approved, AST PASS, verify 0 errors).
