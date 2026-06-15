# Final Comprehensive Review: sys-address-mod (gpt-5.5)

## Checklist
- [x] Read required source/spec/proof/log/TCB files.
- [x] Read required skills: `spec-design`, `verus-constraints`, `ast-consistency`, `bug-reporting`.

### Caller Analysis
- [x] Read `caller_analysis.md`.
- [ ] Every caller success/failure expectation is covered by external-top contracts. **FAIL**: `from_raw_value` lacks a bidirectional/liveness/domain predicate; `is_aligned` lacks explicit consistency with `align_up`/`align_down`.

### View Design
- [x] Read `view_design.md`.
- [ ] Final code matches the design without forbidden constructs. **FAIL**: the final abstraction uses `pub uninterp spec fn spec_addr<T>` instead of a concrete View-backed projection; `uninterp spec fn` is banned by the required skills.

### Specification
- [x] In-scope trait method declarations have `#[verus_spec]` contracts.
- [ ] Contracts are complete and non-redundant. **FAIL**: `from_raw_value` is one-sided and allows spurious `Err(BadAddress)`; `addr_inv(&a)` on `Ok` is logically subsumed by `spec_addr(&a) == raw_addr as int` plus `raw_addr: usize`.

### Proving
- [x] In-scope items are trait method declarations and therefore have no bodies of their own.
- [ ] Implementation obligations are fully discharged/trusted. **FAIL**: the local `impl Address for VirtualAddress` methods remain unverified (seen in verify-sys coverage output), and downstream assume-specification boundaries still exist for some implementors.

### Cheating Elimination
- [x] `admit()`: 0.
- [x] `assume(...)`: 0.
- [x] `external_body`: 0 in the address module files.
- [x] `assume_specification`: 0 in the address module files.
- [x] cfg-gated exec cheating: 0 per `make verify-sys` guardrail check.
- [ ] Forbidden spec escapes eliminated. **FAIL**: `uninterp spec fn spec_addr<T>` remains at `mod.spec.rs:41`.

### Bug Recording
- [x] Read `bugs.md`.
- [ ] Reconciled with final state. **FAIL**: the environment/toolchain mismatch marked resolved in `bugs.md` reproduced for bare `make verify-sys`; additional spec/guardrail issues found here are not recorded as code bugs (they are verification/specification blockers, not true exec-code bugs per bug-reporting).

## Spec Quality

In-scope target functions are the `pub trait Address` method declarations:

- `from_raw_value(raw_addr: usize) -> Result<Self, Error>` (`mod.rs:59-66`)
- `into_raw_value(self) -> usize` (`mod.rs:72-76`)
- `is_aligned(&self, align: Alignment) -> Result<bool, Error>` (`mod.rs:130-135`)

Findings:

1. **`from_raw_value` is under-specified on failure and liveness.**
   Current contract:
   ```rust
   ensures match result {
       Ok(a) => spec_addr(&a) == raw_addr as int && addr_inv(&a),
       Err(e) => e.code == crate::error::ErrorCode::BadAddress,
   }
   ```
   This says what an `Ok` value means and pins the error code, but it does not state *when* success is required or *why* failure is allowed. A conforming implementation could return `Err(BadAddress)` for every input and still satisfy the trait contract. This violates the spec-design liveness/bidirectional-failure guidance and misses caller expectations that `from_raw_value(a.into_raw_value())` succeeds for valid addresses.

2. **`from_raw_value` has a subsumed `Ok` conjunct.**
   `addr_inv(&a)` is `0 <= spec_addr(&a) <= usize::MAX as int`. When `spec_addr(&a) == raw_addr as int` and `raw_addr: usize`, that bound is already implied. This is redundant, although directly useful to callers.

3. **`into_raw_value` is mostly good as an external-top projection.**
   `ensures result as int == spec_addr(&self)` is meaningful and directly useful for pointer/address arithmetic. However, because `spec_addr` is uninterpreted and not tied to `View` by a concrete definition, this relies on implementor obligations that are not fully discharged in this module.

4. **`is_aligned` covers its boolean payload and totality.**
   Current contract guarantees `result is Ok` and the returned bool equals `addr_is_aligned(spec_addr(self), align)`. This covers the main caller guard pattern. It does not state consistency with `align_up`/`align_down`, which caller analysis listed as an expectation, but those methods are out of scope and unspecced here.

5. **Forbidden `uninterp` remains.**
   `mod.spec.rs:41` defines `pub uninterp spec fn spec_addr<T>(addr: &T) -> int;`. The required `spec-design`/`verus-constraints` skills ban `uninterp spec fn` except for a narrow external-body View consequence that does not apply here. This is a verification/specification blocker.

## Caller Coverage  (Covered: 6/11; Missing)

Strict coverage against `caller_analysis.md`:

1. `from_raw_value` Ok returns raw identity (`a@ == raw` / projection equals raw): **covered** via `spec_addr(&a) == raw_addr as int`.
2. `from_raw_value` Ok returns a valid address of this type, including refined domain invariants: **missing/partial**. Only universal pointer bound is stated; no type-specific domain predicate is available.
3. `from_raw_value` Err is `Error::BadAddress`: **covered** via `e.code == BadAddress`.
4. `from_raw_value` Err iff value violated the domain / no spurious failure: **missing**.
5. `from_raw_value` is inverse of `into_raw_value`, especially `from_raw_value(a.into_raw_value()) == Ok(a)` for valid addresses: **missing/partial**. The two one-way projection facts do not guarantee success or equality for re-wrapping an existing address.
6. Generic delegation through `T: Address` has usable contracts: **covered/partial** for projection facts, limited by missing domain/liveness.
7. `from_raw_value` errors propagate cleanly with `?`: **covered** by `Result` plus BadAddress error code.
8. `into_raw_value` returns exact raw numeric address: **covered** via `result as int == spec_addr(&self)`.
9. `into_raw_value` is total/never fails: **covered** by signature.
10. `is_aligned` returns `Ok(true)` iff address is a multiple of `align`: **covered** via `result is Ok` and `result->Ok_0 == addr_is_aligned(...)`.
11. `is_aligned` is consistent with `align_up`/`align_down`: **missing** in this trait contract set.

Primary missing items: a type-domain predicate for raw construction, bidirectional success/failure semantics for `from_raw_value`, a round-trip/liveness theorem for `from_raw_value(into_raw_value(...))`, and alignment consistency with the out-of-scope aligners.

## Proof Completeness

Commands/run outputs:

```text
$ python3 /home/ruize/verus-ai-exp/verus-ai/scripts/fn_coverage.py src/libs/sys/src/sys/mm/address/mod.rs src/libs/sys/src/sys/mm/address/mod.rs
# Function Coverage Report
- Source exec fns: 0
- Verus exec fns: 0
- Verus spec fns: 0
- Verus proof fns: 0
- Matched: 0
- Missing: 0
- Extra: 0
```

This matches the note that the target items are trait method declarations, not free exec functions. It also means the helper does not prove implementation coverage for the trait methods.

Raw guardrail search, comments stripped:

```text
[target_mod_files]
admit=0
assume=0
external_body=0
assume_specification=0
cfg_attr=0
cfg=2
  src/libs/sys/src/sys/mm/address/mod.rs:9: #[cfg(verus_keep_ghost)]
  src/libs/sys/src/sys/mm/address/mod.rs:11: #[cfg(verus_keep_ghost)]
uninterp=1
  src/libs/sys/src/sys/mm/address/mod.spec.rs:41: pub uninterp spec fn spec_addr<T>(addr: &T) -> int;

[address_dir_files]
admit=0
assume=0
external_body=0
assume_specification=0
cfg_attr=0
cfg=6
  src/libs/sys/src/sys/mm/address/mod.rs:9: #[cfg(verus_keep_ghost)]
  src/libs/sys/src/sys/mm/address/mod.rs:11: #[cfg(verus_keep_ghost)]
  src/libs/sys/src/sys/mm/address/virt.rs:9: #[cfg(verus_keep_ghost)]
  src/libs/sys/src/sys/mm/address/virt.rs:11: #[cfg(verus_keep_ghost)]
  src/libs/sys/src/sys/mm/address/virt.rs:39: #[cfg(target_pointer_width = "32")]
  src/libs/sys/src/sys/mm/address/virt.rs:308: #[cfg(target_pointer_width = "32")]
uninterp=1
  src/libs/sys/src/sys/mm/address/mod.spec.rs:41: pub uninterp spec fn spec_addr<T>(addr: &T) -> int;
```

`admit()` count is 0, so there is no admit blocker. `external_body` count is 0 in target/address files.

However, proof completeness is still not clean because the local trait implementor methods are not verified. `verify-sys` coverage output includes the `VirtualAddress` trait methods (`from_raw_value`, `is_aligned`, `into_raw_value`) among unverified functions, and the source comment at `virt.rs:260-268` states that `into_raw_value` remains covered downstream by `assume_specification`.

## TCB Compliance

- `external_body` in `src/libs/sys/src/sys/mm/address/mod.rs`: 0.
- `external_body` in `src/libs/sys/src/sys/mm/address/mod.spec.rs`: 0.
- `external_body` in `src/libs/sys/src/sys/mm/address/mod.proof.rs`: 0.
- `external_body` in entire `src/libs/sys/src/sys/mm/address/` directory: 0.

Therefore there is no address-module `external_body` that needs a TCB-allowed entry; TCB compliance for `external_body` is vacuously satisfied.

The TCB list does allow several downstream `assume_specification` boundaries for `sys::mm::Address`/implementors, but those are outside this module. They remain follow-on trust boundaries, not direct violations of the address module files.

## Guardrails Compliance  (admit, assume, external_body, assume_specification, cfg-gated exec counts)

Exact counts, comments stripped, in target `mod.*` files:

| Dimension | Count | Locations |
|---|---:|---|
| `admit()` | 0 | none |
| `assume(...)` | 0 | none |
| `external_body` | 0 | none |
| `assume_specification` | 0 | none |
| cfg-gated exec cheating | 0 | none reported by `make verify-sys`; `#[cfg(verus_keep_ghost)]` include gates at `mod.rs:9,11` are spec/proof includes, not exec bodies |
| `cfg` attributes total (informational, target files) | 2 | `mod.rs:9`, `mod.rs:11` |
| `uninterp spec fn` | 1 | `mod.spec.rs:41` |

Exact counts across the full address directory:

| Dimension | Count | Locations |
|---|---:|---|
| `admit()` | 0 | none |
| `assume(...)` | 0 | none |
| `external_body` | 0 | none |
| `assume_specification` | 0 | none |
| cfg-gated exec cheating | 0 | none reported by guardrail check |
| `cfg` attributes total (informational) | 6 | `mod.rs:9,11`; `virt.rs:9,11`; `virt.rs:39,308` |
| `uninterp spec fn` | 1 | `mod.spec.rs:41` |

Blocker: `uninterp spec fn` is forbidden by the required skills even though it is not one of the five requested numeric guardrail dimensions.

## AST Consistency

Command/output:

```text
$ python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/libs/sys/src/sys/mm/address/mod.rs
✅ All exec functions consistent.
# AST Consistency Report: ast_orig_dmbqgwo2

**Source:** `/tmp/ast_orig_dmbqgwo2.rs`
**Verus:** `src/libs/sys/src/sys/mm/address/mod.rs`

## Summary

- Functions matched: 0/0
- Functions mismatched: 0
- Missing in Verus: 0
- Extra in Verus: 0
- **Consistent: YES**
```

No `// VERUS REWRITE`, `// VERUS DEVIATION`, or `// VERUS BUG FIX` comments were present in `src/libs/sys/src/sys/mm/address/`; therefore there are no rewrite comments to manually validate. AST consistency reports no mismatch, but it matches 0 functions because the in-scope items are trait method declarations.

Spec drift command/output:

```text
$ python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff src/libs/sys/src/sys/mm/address/mod.rs --before HEAD
# Spec Drift Report

## Summary

- Functions with changes: 0
- Contract drift (⚠ review required): 0
  - Ensures removed: 0
  - Requires added: 0
- Proof drift (informational): 0
- Functions added: 0
- Functions removed: 0

✅ No contract drift detected.
```

## Verification

Bare required command from `/home/ruize/nanvix-phy`:

```text
$ make verify-sys
Using Verus installation at /home/ruize/toolchain/verus.
...
Checking vstd v0.0.0-2026-05-31-0205
error: expected generics to match:
        expected
        found u8
  --> .../vstd-0.0.0-2026-05-31-0205/std_specs/atomic.rs:16:9
...
error: could not compile `vstd` (lib) due to 9 previous errors

=== Results ===
  0 verified
  compilation/setup error (verus did not run)
  Exit code : 101
...
=== Summary ===
  verification: 0 verified, compilation/setup error (verus did not run) (exit 101)
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 2/254 exec functions have contracts
  status: VERIFY_FAILED
make: *** [Makefile:617: verify-sys] Error 101
```

Toolchain diagnosis:

```text
build/verus-version: 0.2026.05.31.5dd6d83
/home/ruize/toolchain/verus/verus --version: 0.2026.06.14.4ea7d0f
```

A follow-up run with the pinned available toolchain succeeds:

```text
$ VERUS_EXECUTABLE_DIR=/home/ruize/toolchain/verus-pinned-0531 make verify-sys
Using Verus installation at /home/ruize/toolchain/verus-pinned-0531.
Checking sys v0.16.17 (/home/ruize/nanvix-phy/src/libs/sys)
verification results:: 6 verified, 0 errors
Finished `dev` profile [optimized + debuginfo] target(s) in 0.98s

=== Results ===
  6 verified
  0 errors
  Exit code : 0

=== Cheating Pattern Check ===
  ✅ No cheating detected.

=== Function Coverage ===
  2/254 exec functions have contracts.
  Unverified function list written to: verus-ai-logs/verify-sys/verus-logs/coverage-unverified.txt

=== Summary ===
  verification: 6 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 2/254 exec functions have contracts
  status: CLEAN
```

Strict result for the requested command is **FAIL** because bare `make verify-sys` did not confirm 0 errors in the current environment. The pinned override demonstrates the code can verify under the correct toolchain, but the requested command currently fails due to the default Verus toolchain mismatch.

## Bug Summary

`bugs.md` entries reconciled:

1. **Code bugs: None found.** Still valid for the three in-scope trait method declarations because they have no bodies. The new issues found in this review are specification/proof-completeness/guardrail issues, not true exec-code bugs under the bug-reporting classification.

2. **Definition cycle when `spec_addr` is bounded by `Address` (resolved).** The cycle is resolved by making `spec_addr<T>` unbounded, but the chosen mechanism is still `uninterp spec fn`, which is forbidden by the required skills. This remains a specification blocker, not a code bug.

3. **Kernel `assume_specification[<VirtualAddress as Address>::into_raw_value]`.** Still present as a downstream/follow-on trust boundary according to comments and TCB notes. It is not in the address module files, but it confirms implementor obligations are not fully eliminated by this module-level trait-spec effort.

4. **Toolchain/`vstd` mismatch marked RESOLVED.** Not resolved in the current default environment: bare `make verify-sys` reproduced the same `vstd/std_specs/atomic.rs` generics mismatch because `/home/ruize/toolchain/verus` is version `0.2026.06.14.4ea7d0f` while the repository pin is `0.2026.05.31.5dd6d83`. The pinned directory `/home/ruize/toolchain/verus-pinned-0531` succeeds when explicitly selected.

No newly discovered true code bug requires a `bugs.md` entry. Surviving issues should be tracked as verification/specification blockers, not as bug-reporting true bugs.

## Issues (highest priority first)

1. **BLOCKER — Required bare verification command fails.**
   `make verify-sys` uses `/home/ruize/toolchain/verus` version `0.2026.06.14.4ea7d0f` against pinned `vstd 0.0.0-2026-05-31-0205`, producing 9 `vstd/std_specs/atomic.rs` compile errors and `VERIFY_FAILED`. The pinned override succeeds, but the requested command does not.

2. **BLOCKER — Forbidden `uninterp spec fn` remains.**
   `src/libs/sys/src/sys/mm/address/mod.spec.rs:41` defines `pub uninterp spec fn spec_addr<T>(addr: &T) -> int;`. The required skills ban uninterpreted spec functions as a verification escape unless they are the narrow external-body View consequence, which this is not.

3. **BLOCKER — `from_raw_value` external-top contract is incomplete.**
   It lacks a domain predicate and bidirectional success/failure/liveness semantics. It allows spurious `Err(BadAddress)` for any input and does not guarantee `from_raw_value(a.into_raw_value())` succeeds for valid addresses.

4. **BLOCKER — Implementor obligations are not fully discharged.**
   Trait declarations have no bodies, but the local `impl Address for VirtualAddress` is not verified; coverage output lists its trait methods among unverified functions. Downstream `assume_specification` boundaries for Address implementors remain documented in the TCB/logs.

5. **MAJOR — `is_aligned` lacks explicit alignment-operation consistency.**
   The boolean predicate is covered, but caller analysis also expects consistency with `align_up`/`align_down`; those out-of-scope trait methods have no contracts here.

6. **MINOR — Redundant `addr_inv` postcondition on successful `from_raw_value`.**
   The universal pointer-sized bound is implied by raw identity plus `raw_addr: usize`. It may aid callers but is technically subsumed.

## Result: FAIL

PASS is not justified because not all checklist items pass. The hard blockers are: bare `make verify-sys` fails in the current environment; a forbidden `uninterp spec fn` remains; `from_raw_value` lacks required liveness/domain/error semantics; and trait implementor obligations are not fully discharged by this module verification.
