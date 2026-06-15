# Final Comprehensive Review (gpt-5.5): arch-frame-number

## Checklist

### Caller Analysis (4)
- [x] Read `caller_analysis.md` and verified the reported public API/caller counts with tool output (`find_callers_output.md`, `rg`).
- [x] Checked all 8 external `FrameNumber::from_raw_value` call sites against the constructor contract.
- [x] Checked all 12 external `FrameNumber::into_raw_value` call sites against the projection contract.
- [ ] Checked constants/sentinel expectations: `MAX` is mirrored by the spec bound, but there is no explicit caller-visible contract/lemma that `FrameNumber::NULL@ == 0` or `FrameNumber::NULL.into_raw_value() == 0`.

### View Design (5)
- [x] `impl View for FrameNumber { type V = int; }` exists.
- [x] `view()` is `closed spec fn` and abstracts to the raw frame index.
- [x] `inv()` is `pub open spec fn` and is marked `#[verifier::type_invariant]`.
- [x] `inv()` states the caller-required range `0 <= self@ <= spec_max_frame_number()`.
- [x] View is minimal, caller-facing, and not a code-as-spec mirror beyond the necessary newtype identity hidden behind the closed view.

### Specification (18)
- [x] `into_raw_value` has an external-top `#[verus_spec]` on the original exec function.
- [x] `into_raw_value` ensures exact projection: `result as int == self@`.
- [x] `into_raw_value` ensures caller-needed range: `0 <= result as int <= spec_max_frame_number()`.
- [x] `into_raw_value` has no unnecessary `requires`; it is total for every valid `FrameNumber` value.
- [x] `from_raw_value` has an external-top `#[verus_spec]` on the original exec function.
- [x] `from_raw_value` has no `requires`; out-of-range input is dynamically handled by `None`.
- [x] `from_raw_value` specifies bidirectional success/failure: `(result is Some) <==> (value as int <= spec_max_frame_number())`.
- [x] `from_raw_value` specifies success preservation: `Some(f) ==> f@ == value as int`.
- [x] `from_raw_value` specifies successful result well-formedness: `Some(f) ==> f.inv()`.
- [x] Error-path meaning is complete: by the iff clause, `None` occurs exactly when `value as int > spec_max_frame_number()`.
- [x] The critical iff is correct against exec `if value > Self::MAX { None } else { Some(...) }`, provided the spec bound mirrors `Self::MAX`.
- [x] `spec_max_frame_number()` mirrors `FrameNumber::MAX`: exec is `mem::MAX_ADDRESS / mem::FRAME_SIZE - 1`; spec is `mem::MAX_ADDRESS as int / spec_frame_size() - 1`, with `spec_frame_size() == mem::FRAME_SIZE as int`.
- [x] Tool-read constants confirm `mem::MAX_ADDRESS = usize::MAX`, `mem::FRAME_SIZE = PAGE_SIZE`, `PAGE_SIZE = 4096`.
- [x] Round-trip identity is derivable from the two contracts: `from_raw_value(v)=Some(f)` gives `f@ == v`, and `into_raw_value(f)` gives the same raw value.
- [x] No tautological success-only spec for `from_raw_value`; the failure path is not omitted.
- [x] The `into_raw_value` range ensure is partially redundant with `result == self@` plus the type invariant (and `usize` non-negativity), but it is directly caller-facing and matches the prior TCB contract; I do not classify it as a blocker.
- [x] Specs are declarative and caller-oriented rather than operational step-by-step code descriptions.
- [ ] `NULL` sentinel identity is not formally exposed by a contract/lemma; only the exec const `pub const NULL: Self = Self(0);` exists, and the view is closed.

### Proving (9)
- [x] `make verify-arch` was run from `/home/ruize/nanvix-phy`.
- [x] Verus exit code was 0.
- [x] Verus status was `CLEAN`.
- [x] Verification was cached/no recompilation, not failed or partial.
- [x] Module files contain `admit`: 0.
- [x] Module files contain `external_body`: 0.
- [x] Module files contain unapproved `external_body`: 0.
- [x] `number.proof.rs` is empty except `verus! { }`; no hidden proof holes were found.
- [x] Tool guardrail output also reported crate-level `assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0` for `verify-arch`.

### Cheating Elimination (12)
- [x] `admit`: 0 in the three module files.
- [x] `assume`: 0 in the three module files.
- [x] `external_body`: 0 in the three module files.
- [x] `assume_specification`: 0 in the three module files.
- [x] `#[verifier::trusted]`: 0 in the three module files.
- [x] `exec_allows_no_decreases`: 0 in the three module files.
- [x] `spinoff_prover`: 0 in the three module files.
- [x] `rlimit`: 0 in the three module files.
- [x] `// VERUS REWRITE`: 0 in the three module files.
- [x] `#[cfg(not(verus_keep_ghost))]`: 0 in the three module files.
- [x] `#[cfg(verus_keep_ghost)]`: 2, both only guard `include!("number.spec.rs")` and `include!("number.proof.rs")`, not exec code.
- [x] `spec_drift.py git-diff ... --before HEAD` reported no contract drift / no spec weakening.

### Bug Recording (5)
- [x] Checked for `bugs.md`; it does not exist.
- [x] Applied bug-reporting classification: missing specs/verification limitations are not true code bugs.
- [x] Assessed in-scope exec bodies from first principles.
- [x] Found no genuine code defect in `from_raw_value` or `into_raw_value` relative to the documented `Self::MAX` range.
- [x] Recorded bug summary in this final review: total recorded 0, true bugs 0.

## Spec Quality

The core external-top contracts for the two in-scope functions are strong and readable.

`FrameNumber::into_raw_value` states the two caller-critical facts:

- exact abstract projection: `result as int == self@`;
- bounded frame index: `0 <= result as int <= spec_max_frame_number()`.

The range clause is logically derivable from the identity plus the type invariant (and the lower bound is also a `usize` domain fact), so it is redundant in a strict minimality sense. I do not treat this as a blocker because caller analysis and the prior TCB boundary explicitly require the range fact in this direct form for no-overflow proofs.

`FrameNumber::from_raw_value` is complete for a dynamic validating constructor. It has no `requires`, and its bidirectional success condition specifies both liveness and failure exactly:

```rust
(result is Some) <==> (value as int <= spec_max_frame_number())
```

This is correct against the exec check:

```rust
if value > Self::MAX { return None; }
Some(Self(value))
```

because `usize` values are non-negative and `spec_max_frame_number()` mirrors `Self::MAX`:

- source: `FrameNumber::MAX = mem::MAX_ADDRESS / mem::FRAME_SIZE - 1` (`number.rs:37`);
- spec: `spec_max_frame_number() = mem::MAX_ADDRESS as int / spec_frame_size() - 1` and `spec_frame_size() = mem::FRAME_SIZE as int` (`number.spec.rs:15-25`);
- constants read by tool: `MAX_ADDRESS = usize::MAX`, `FRAME_SIZE = PAGE_SIZE`, `PAGE_SIZE = 4096` (`constants.rs:48-108`).

The `View` is appropriate: `type V = int`, closed `view`, and `inv()` exposes the caller-needed range bound. The remaining quality gap is not in the two function contracts but in the constant/sentinel expectation: there is no explicit formal statement that `FrameNumber::NULL@ == 0` or that `FrameNumber::NULL.into_raw_value() == 0`. The source constant is visibly `Self(0)`, but the view is closed and no exported contract/lemma records the sentinel identity for callers.

## Caller Coverage

Covered 23/24 call-site/semantic expectations. Missing list below.

Covered:

1. `from_raw_value` external call sites (8/8): `pde.rs:303`, `pte.rs:304`, `page_table.rs:589`, `identity_map.rs:530`, `identity_map.rs:593`, `mm/phys/frame.rs:148`, `mm/phys/frame.rs:223`, `phys.rs:211`. The iff contract covers success/failure for all dynamic raw indices.
2. `into_raw_value` external call sites (12/12): `page_table.rs:584`, `pde.rs:320,375`, `pte.rs:321,362`, `phys.rs:159`, `mm/phys/frame.rs:291,369,429,482,518,569`. The identity and range ensures cover exact projection and no-overflow enablement.
3. `MAX`: covered by `spec_max_frame_number()` mirroring the exec `FrameNumber::MAX` expression.
4. Round-trip identity: covered derivationally by `from_raw_value` success preservation plus `into_raw_value` identity.
5. Range/no-overflow enablement: covered by `into_raw_value` range ensure and `FrameNumber::inv()`.
6. Totality/purity: covered by no `requires`, straight-line verified bodies, no mutation/global state, and `make verify-arch` exit 0.

Missing:

- `NULL` sentinel identity: no explicit contract/lemma exposes `FrameNumber::NULL@ == 0` or `FrameNumber::NULL.into_raw_value() == 0` to callers. Tool search found only `number.rs:39: pub const NULL: Self = Self(0);` and no `NULL@`, `NULL.into_raw_value`, `spec_null`, or equivalent proof/spec in the three in-scope files.

## Proof Completeness

- `admit`: 0.
- `external_body`: 0.
- `external_body` not in TCB: 0.

Exact module-file guardrail count output:

```text
admit: 0
assume: 0
external_body: 0
assume_specification: 0
cfg_attr_not_verus_keep_ghost: 0
cfg_verus_keep_ghost: 2
  src/libs/arch/src/x86/mem/paging/frame/number.rs:9:#[cfg(verus_keep_ghost)]
  src/libs/arch/src/x86/mem/paging/frame/number.rs:11:#[cfg(verus_keep_ghost)]
cfg_not_verus_keep_ghost: 0
verus_rewrite: 0
trusted: 0
exec_allows_no_decreases: 0
spinoff_prover: 0
rlimit: 0
```

## TCB Compliance

YES. There are no `external_body` occurrences in the three in-scope module files, so there are no unapproved trust boundaries to compare against `tcb-allowed.md`.

The TCB allow-list still records older cross-module `assume_specification` boundaries for `FrameNumber::into_raw_value` / `from_raw_value` in `phys.spec.rs`, but this module itself introduces no `external_body` or `assume_specification`.

## Guardrails Compliance

Module-file counts:

- `admit`: 0
- `assume`: 0
- `external_body`: 0
- `assume_specification`: 0
- cfg-gated exec code: 0
- non-exec `#[cfg(verus_keep_ghost)]` include gates: 2 (`number.rs:9`, `number.rs:11`)
- `// VERUS REWRITE`: 0
- `#[verifier::trusted]`: 0
- `exec_allows_no_decreases`: 0
- `spinoff_prover`: 0
- `rlimit`: 0

`spec_drift.py` output:

```text
# Spec Drift Report

## Summary

- Functions with changes: 0
- **Contract drift (⚠ review required): 0**
  - Ensures removed: 0
  - Requires added: 0
- Proof drift (informational): 0
- Functions added: 0
- Functions removed: 0

**✅ No contract drift detected.**
```

## AST Consistency

PASS.

Commands run:

```bash
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref HEAD src/libs/arch/src/x86/mem/paging/frame/number.rs count
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref HEAD src/libs/arch/src/x86/mem/paging/frame/number.rs summary
rg 'VERUS REWRITE' <three in-scope files>
```

Tool output:

```text
✅ Consistent: 4 functions, 1 structs match.
## Functions

Function                                 Status               Verification
──────────────────────────────────────── ──────────────────── ────────────────
FrameNumber::from_raw_value              MATCH
FrameNumber::into_raw_value              MATCH
test_frame_number_from_raw_value_max     MATCH
test_frame_number_from_raw_value_zero    MATCH

## Structs

Struct                                   Status
──────────────────────────────────────── ────────────────────
FrameNumber                              MATCH

Consistent: ✅ YES (matched=4 mismatched=0 missing=0 extra=0)
```

`VERUS REWRITE`: no matches. Semantic rewrite review: N/A.

## Verification

verus: PASS.

`make verify-arch` from `/home/ruize/nanvix-phy` exited 0 and reported CLEAN:

```text
=== Results ===
  cached (no recompilation)
  —
  Exit code : 0

=== Cheating Pattern Check ===
  ✅ No cheating detected.

=== Function Coverage ===
  2/525 exec functions have contracts.
  Unverified function list written to: verus-ai-logs/verify-arch/verus-logs/coverage-unverified.txt

=== Summary ===
  verification: cached (no recompilation), — (exit 0)
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 2/525 exec functions have contracts
  status: CLEAN
```

## Bug Summary

- `bugs.md`: absent (`verus-ai-logs/nanvix-phys-arch-frame-number/bugs.md` does not exist).
- Total recorded: 0.
- True Bugs: 0.

From first principles, the in-scope exec code has no genuine code defect relative to the documented `FrameNumber` range: `from_raw_value` returns `None` exactly above `Self::MAX`, returns `Some(Self(value))` otherwise, and `into_raw_value` returns the stored newtype value. The missing `NULL` formal identity is a specification/coverage gap, not a code bug.

## Issues (highest priority first)

1. **Missing caller-visible `NULL` sentinel identity (BLOCKER for final PASS).**
   - Caller analysis states that `FrameNumber::NULL` users assume `NULL.into_raw_value() == 0` and that `NULL` is a valid in-range sentinel.
   - Validity is covered by the type invariant, but zero identity is not explicitly exposed by any in-scope `requires`/`ensures`, spec function, or lemma.
   - Tool search result: only `number.rs:39: pub const NULL: Self = Self(0);`; no `NULL@`, `NULL.into_raw_value`, `spec_null`, or equivalent proof/spec in `number.rs`, `number.spec.rs`, or `number.proof.rs`.
   - This is not an `admit`/TCB/verification failure and not a code bug. It is a caller-coverage/spec completeness gap.

## Result: FAIL

Verification and guardrails are clean, but strict final approval requires every caller-analysis expectation to have corresponding formal coverage. The `NULL` sentinel identity expectation is not formally exposed, so the checklist is not fully satisfied.
