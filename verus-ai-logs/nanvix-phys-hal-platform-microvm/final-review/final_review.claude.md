# Final Comprehensive Review: hal-platform-microvm (claude-opus-4.8)

Reviewer: independent strict final review (claude-opus-4.8)
Date: 2026-06-15
Branch: `verus-ai-prove-bottom-up` (HEAD `bb3c9c7b2`)
In-scope target: `gva_to_gpa` in `src/kernel/src/hal/platform/microvm/mod.rs`
(all other module functions OUT OF SCOPE).

## Checklist

### Caller Analysis
- [x] `caller_analysis.md` exists and identifies the real caller(s).
- [x] Script false-negative explained (re-export via `crate::hal::platform`).
- [x] Sole in-tree caller located: `mm/phys/mod.rs:114` (`book_mmio_regions`).
- [x] Trait obligations assessed (none — free function).
- [x] Caller expectations enumerated (totality/infallibility, determinism, frame correspondence, identity).

### View Design
- [x] `view_design.md` exists and justifies abstraction choice.
- [x] No stateful View / no `impl View` (pure stateless function) — justified.
- [x] Abstraction = the translation map as one `open spec fn` over `int`.
- [x] Substitution/minimality test applied; all candidate struct fields rejected.
- [x] `open` vs `closed` decision justified (identity must be visible to caller).
- [x] Rejected alternatives documented.

### Specification
- [x] `#[verus_spec]` added in-place on the original exec function (signature unchanged).
- [x] Single `ensures result as int == spec_gva_to_gpa(gva as int)` (identity map).
- [x] No `requires` — totality/infallibility faithfully recorded.
- [x] `spec_gva_to_gpa` is `pub open` with a concrete definition (no `uninterp`).
- [x] No tautological / subsumed / one-sided / operational clauses.
- [x] Spec is caller-usable and derivable from signature + module purpose.

### Proving
- [x] 0 `admit()` in the module.
- [x] 0 `external_body` in the module.
- [x] `make verify-kernel MODULE=hal::platform::microvm` → `1 verified, 0 errors`.
- [x] `mod.proof.rs` is empty (`verus! { }`) — no proof debt.

### Cheating Elimination
- [x] admit = 0 (module-scoped).
- [x] assume = 0 (module-scoped).
- [x] external_body = 0 (module-scoped).
- [x] assume_specification = 0 (module-scoped).
- [x] cfg-gated exec = 0 (only the two standard `#[cfg(verus_keep_ghost)] include!` lines).
- [x] No `verifier::trusted` / `external` / `exec_allows_no_decreases` / `spinoff_prover` / `rlimit` / `uninterp`.
- [x] AST consistency PASS (exec code unchanged).
- [x] Spec-drift PASS (no weakening).

### Bug Recording
- [x] `bugs.md` absent; no bugs found — equivalent to "None".
- [x] In-scope function reconciled against final code (identity is correct).
- [x] No undocumented bugs discovered.

## Spec Quality

The external-top contract is correct, complete, and understandable.

```rust
#[verus_spec(result =>
    ensures
        result as int == spec_gva_to_gpa(gva as int),
)]
pub fn gva_to_gpa(gva: usize) -> usize { gva }

pub open spec fn spec_gva_to_gpa(gva: int) -> int { gva }   // identity
```

Assessed against the spec-design principles:

- **Bound to exec code** ✅ — `ensures` annotates the real exec function in-place;
  not a copy.
- **Sufficient to reject bugs** ✅ — only `return gva` satisfies `result == gva`;
  `gva+1`, `0`, or any remap fails. The spec is not satisfiable by a buggy impl.
- **Declarative / WHAT not HOW** ✅ — the indirection through `spec_gva_to_gpa`
  names the platform GVA→GPA translation map rather than restating the body. A
  future non-identity platform redefines a single hook.
- **Independent from code** ✅ — derivable from signature + "MicroVM runs the
  guest identity-mapped" alone, without reading the body.
- **Totality** ✅ — no `requires`; defined for every `usize`, faithfully recording
  the caller's unguarded loop call.
- **open vs closed** ✅ — `open` is correct: the caller must derive frame
  correspondence, which on MicroVM *is* `result == gva`; hiding it behind `closed`
  would defeat the only reason to verify the function. Exposure is contract, not leak.
- **int typing** ✅ acceptable — `int` is used in the spec fn / cast; identity
  cannot overflow and the cast equality cleanly implies `result == gva`.
  (Minor, non-blocking: spec-design Part 1 #7 expresses a soft preference for
  `usize` on addresses; here `int` is harmless and standard.)
- **Minimality / no subsumed clauses** ✅ — determinism, injectivity, and
  frame-stepping are corollaries of `result == gva` and are correctly documented
  as derivable rather than emitted as redundant ensures.
- **No tautology / no one-sided error spec** ✅ — single non-trivial ensures;
  no error path exists.

Verdict: **high-quality spec**, no defects.

## Caller Coverage  (Covered: 3/3 invariants + identity; Missing: none)

From `caller_analysis.md` ("Key Invariants" + "Caller Expectations"):

| Caller expectation | Mapped to spec | Covered |
|---|---|---|
| Totality / infallibility (no `Result`, no panic/trap) | No `requires`; total `usize`→`usize`; identity has no panic/trap path | ✅ |
| Purity / determinism (same input → same output) | `spec_gva_to_gpa` is a pure math function of `gva` alone | ✅ (corollary) |
| Frame correspondence (per-`FRAME_SIZE` stepping) | `result == gva` ⇒ advancing GVA by FRAME_SIZE advances GPA equally | ✅ (subsumed) |
| Identity on MicroVM (`gpa == gva`) | `result as int == spec_gva_to_gpa(gva) == gva`, `open` body | ✅ |
| Injectivity / no spurious remap | `result == gva` ⇒ injective | ✅ (subsumed) |
| Post-failure behavior | N/A — no failure path (caller's `?` is on the *subsequent* `from_mmio_address`) | ✅ (correctly N/A) |

Every caller expectation maps to the spec. **Covered: all; Missing: none.**

## Proof Completeness (Remaining admit(): 0; Remaining external_body not in tcb-allowed: 0)

- `grep -rn "admit(" src/kernel/src/hal/platform/microvm` → 0 hits.
- `grep -rn "external_body" src/kernel/src/hal/platform/microvm` → 0 hits.
- `mod.proof.rs` contains only `verus! { }`.

No in-scope `admit()` (would be a BLOCKER) and no module `external_body`.

## TCB Compliance

`verus-ai-logs/tcb-allowed.md` does **not** list any function in
`hal/platform/microvm`. The module contains **0** `external_body`, so nothing
needs to be on the list. **Compliant.**

The `make` summary's `external_body=11`, `admit=27`, `cfg_gate=14` are
**crate-wide** totals produced by OTHER, out-of-scope modules (`mm/phys/*`,
`arch/x86/mem/paging/*`, `bump_allocator`, etc., all enumerated in
`tcb-allowed.md`). Directory-scoped grep confirms microvm contributes **0** to
each. Module-scoped vs crate-wide distinction verified.

## Guardrails Compliance

**Module-scoped (microvm/ directory) — all zero:**
admit: 0, assume: 0, external_body: 0, assume_specification: 0, cfg-gated exec: 0.

(The only `#[cfg(verus_keep_ghost)]` occurrences are the two standard
`include!("mod.spec.rs")` / `include!("mod.proof.rs")` spec/proof include
guards at `mod.rs:9,11` — explicitly excluded by the task. No exec branch,
expression, match arm, or function is cfg-gated.)

**Crate-wide totals (out-of-scope, informational):**
assume: 0, external_body: 11, admit: 27, trusted: 0, no_decreases: 0, cfg_gate: 14.
All originate from other modules already governed by `tcb-allowed.md`.

admit (in scope) = 0 and assume (in scope) = 0 → no BLOCKER.

## AST Consistency (PASS)

`ast_consistency.py --base-ref b52e0c915 src/.../microvm/mod.rs count`
→ `✅ Consistent: 28 functions, 3 structs match.` (`gva_to_gpa` = MATCH).
No `// VERUS REWRITE`, `// VERUS DEVIATION`, or `// VERUS BUG FIX` comments exist
in the module (none required — exec code is byte-for-byte semantically unchanged).
**PASS.**

## Verification (PASS)

Command: `make verify-kernel MODULE=hal::platform::microvm` (forced fresh run).

Exact summary line:
```
verification results:: 1 verified, 0 errors (partial verification with `--verify-*`)
=== Summary ===
  verification: 1 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=11 admit=27 trusted=0 no_decreases=0 cfg_gate=14
  coverage: 1/31 exec functions have contracts
  status: CLEAN
```

The single verified function is the in-scope `gva_to_gpa`. Coverage `1/31` is
expected and correct: the task scope is exactly one function; the other 30 are
out of scope. **PASS.**

## Bug Summary

`bugs.md` does not exist → effectively "None". Reconciliation of the in-scope
function against final code: `gva_to_gpa` returns `gva` (identity); the spec
pins `result == gva`; this is the documented MicroVM identity-map contract — the
code is correct. No true bugs, no context-dependent issues, no undocumented bugs
found. **Bug Summary: None.**

## Issues (highest priority first)

1. (Minor / non-blocking) The spec fn `spec_gva_to_gpa` and the comparison use
   `int` rather than `usize`. spec-design Part 1 #7 expresses a soft preference
   for `usize` on address-typed values. Here it is harmless: the identity cannot
   overflow and `result as int == gva as int` cleanly implies `result == gva`.
   No action required.

No correctness, soundness, coverage, cheating, drift, or AST issues found.

## Result: PASS

All checklist items pass. The single in-scope function `gva_to_gpa` is fully
verified (`1 verified, 0 errors`) with a correct, complete, minimal external-top
contract; zero in-scope cheating (admit/assume/external_body/assume_specification/
cfg-gated exec all 0); TCB-compliant; AST-consistent; no spec drift; no bugs.
The reported crate-wide cheating counts are entirely from out-of-scope modules.
