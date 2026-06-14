# Final Comprehensive Review: sys-virt-address

> Consolidated from two independent sub-agent reviews:
> - `final_review.claude.md` (claude-opus-4.8)
> - `final_review.gpt5codex.md` (gpt-5.3-codex)
>
> Orchestrator adjudicated the two disagreements (caller coverage of
> `into_raw_value`; AST/spec-drift "mismatches") against ground-truth tool
> output. Both reviewers independently concluded **FAIL**.

## Checklist

### Caller Analysis
- [x] All pub functions have callers searched (tool-verified) — `find_callers_lsp_output.md` (LSP `find_callers_lsp.py`); `new` 8, inherent `from_raw_value` 5, `Address::into_raw_value` 3, type 32 refs.
- [x] Caller expectations (success + failure) documented — `caller_analysis.md:35-91`; all in-scope fns are total/infallible (no failure arm).
- [x] Abstract resource identified — "virtual address as a single `int`" (`caller_analysis.md:93-97`).
- [x] Pre-existing specs assessed — `virt.spec.rs`/`virt.proof.rs` empty (`verus! { }`); no upstream `#[verus_spec]` (`caller_analysis.md:113-126`).

### View Design
- [x] Every field passes the substitution test — single `self@ : int` survives any rewrite (`view_design.md:187-206`).
- [x] All caller-observable state represented — the lone observable is the raw address integer.
- [x] No implementation-specific fields — `closed view()` hides the `.0` field.
- [x] inv() encodes real constraints (not trivially true) — ACCEPTABLE/N-A: `VirtualAddress` is a **total** newtype with no semantic invariant; no `inv()` is defined and none is required (`view_design.md:96-123`). (codex flagged the absent `inv()`; it is intentional per the view design and is **not** a blocker.)
- [x] Mathematical types used — `type V = int`.

### Specification
- [ ] **Every in-scope exec function has requires/ensures** — **FAIL.** `make verify-sys` coverage = **2/255**; only `VirtualAddress::new` (`virt.rs:48-53`) and inherent `from_raw_value` (`virt.rs:65-70`) carry `#[verus_spec]`. The in-scope **`VirtualAddress::into_raw_value`** (`virt.rs:253`) has no inline spec and its enclosing block `impl Address for VirtualAddress` (`virt.rs:167`) is **not** `#[verus_verify]`-annotated, so the impl body is NOT verified (confirmed: it appears in `verus-logs/coverage-unverified.txt`). **BLOCKER.**
- [ ] **Caller coverage: every caller expectation has corresponding requires/ensures** — **FAIL, 3/4.** `into_raw_value` callers (`mm/mmio.rs:67`, `pm/sync.rs:37,65`) depend on the round-trip inverse `result == self@`; that contract is declared only at the trait (`mod.rs:63-66`) and is **not proven** for the `VirtualAddress` impl.
- [x] View consistency — the two written specs reference `self@`/`result@` and the `int` View.
- [x] No tautological ensures — both specs are exact value equalities.
- [x] No subsumed ensures.
- [x] Error paths have meaningful ensures — N/A: all three in-scope fns are total (no `Err`/`Option`).
- [x] No assume_specification for workspace-internal code — none present (count 0).
- [x] vstd searched before any assume_specification — N/A (none used).
- [x] Specs written for the caller — `new`/`from_raw_value` ensures are directly usable.
- [ ] **Trait obligations satisfied** — **FAIL.** The `Address` trait declares `into_raw_value` (`ensures result as int == self@`, `mod.rs:63-66`), but the `VirtualAddress` impl is not verified to meet it (impl block lacks `#[verus_verify]`).
- [x] Spec completeness (advisory) — the two specced fns are complete; nondeterminism N/A.
- [x] Loop invariants — N/A (no loops in scope).
- [x] No cheating on module's own functions — `admit=0 assume=0 external_body=0 trusted=0` in target files.
- [x] No specs weakened — `spec_drift.py` "ensures removed" is a **false positive** (inherent-vs-trait `from_raw_value` name collision); `git diff ca7e88be8 HEAD` shows specs only **added** to a previously empty baseline.
- [x] Bug awareness — no fundamentally incorrect code; `bugs.md` correctly absent.
- [ ] Cross-module regression (`make verify`) — NOT run in this review (other modules carry known pre-existing cheating per git log; out of this module's scope). `make verify-sys` passes. Not the blocker, but item unchecked.
- [x] Verification: `make verify-sys` PASS (exit 0, `status: CLEAN`, fresh non-cached "6 verified, 0 errors"); `make build` OK.

### Proving
- [x] No specs weakened — see spec_drift note above (false positive).
- [x] Zero remaining admit() — 0.
- [x] Zero external_body unless TCB-listed — 0 external_body in target files.
- [x] Zero assume/assume_specification — 0.
- [x] No cfg-gated exec code — only `#[cfg(verus_keep_ghost)]` on the two `include!` lines (`virt.rs:9,11`), not exec.
- [x] Cheating audit counts reported — all 0 (see Guardrails).
- [x] Claimed Verus limitations have isolated reproducers — N/A (none claimed; no `// VERUS REWRITE`).
- [x] Exec rewrites minimal/equivalent — no exec rewrites (AST: no exec body changed).
- [ ] Cross-module regression (`make verify`) — not run (see above).
- [x] Verification `make verify-sys` + `make build` — 0 errors, 0 warnings.

### Cheating Elimination
- [x] Zero admit() — 0.
- [x] Zero assume() — 0.
- [x] Zero trusted functions — 0.
- [x] Zero exec_allows_no_decreases_clause — 0.
- [x] Zero cfg-gated exec code — 0 (only `include!` cfg).
- [x] Zero external_body unless TCB-listed — 0.
- [x] AST consistency: zero **real** mismatches — the reported `from_raw_value` MISMATCH is a name-collision artifact (`diff --name` returns MATCH); `clone_address` EXTRA_IN_VERUS is a false positive (pre-existed at `ca7e88be8^:virt.rs:241`, byte-identical body). No exec body changed.
- [x] All exec rewrites have VERUS REWRITE comment — N/A (none).
- [x] For each surviving external_body confirm TCB-listed — N/A (none).
- [x] No specs weakened — false-positive spec_drift; verified additive.
- [ ] Cross-module regression (`make verify`) — not run.
- [x] Verification — `make verify-sys` 0 errors.

### Bug Recording
- [x] bugs.md exists if bugs were found — no bugs found; correctly absent.
- [x] Each bug is a real code defect — N/A.
- [x] Each bug entry has What/Why/How-Verus-Helped/Severity/Fix — N/A.
- [x] No external_body used to mask a defect — N/A.
- [x] Bug entries include provenance — N/A.

## Spec Quality
The two written external-top specs are correct, minimal, and caller-usable:
`new` → `result@ == value as int`; inherent `from_raw_value` → `result@ == raw_addr as int`.
No tautological, subsumed, one-sided, or operational-code-as-spec anti-patterns.
The `int` View matches the abstract resource and mirrors the sibling address
tower. **However the API surface is incomplete**: the in-scope projection
`into_raw_value` — the documented inverse of construction and the basis of the
round-trip identity callers rely on — has no **verified** contract on the
`VirtualAddress` impl.

## Caller Coverage
- Covered: **3 / 4**
- Missing: **`VirtualAddress::into_raw_value`** (`Address::into_raw_value` impl, `virt.rs:253`). The trait declares the contract (`mod.rs:63-66`) but the impl block (`virt.rs:167`) is not `#[verus_verify]`-annotated, so the body is unverified (listed in `coverage-unverified.txt`). Callers `mm/mmio.rs:67` and `pm/sync.rs:37,65` therefore have no **proven** `from_raw_value(x).into_raw_value() == x` round-trip.

## Proof Completeness
- Remaining admit(): **0**.
- Remaining external_body not in tcb-allowed.md: **0**.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** (vacuous — zero `external_body` in target files).

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**, cfg-gated exec: **0**.

## AST Consistency
- AST check: **PASS** (no exec code changed). The summary's single `from_raw_value` MISMATCH and the `clone_address` EXTRA_IN_VERUS are tooling false positives from duplicate inherent/trait method names; `git diff ca7e88be8 HEAD -- virt.rs` confirms only spec annotations were added, every exec body byte-identical. (codex's report of 3 mismatches on `align_up/align_down/is_aligned` used a wrong base ref and is superseded.)

## Verification
- verus (`make verify-sys`): **PASS** — exit 0, fresh "6 verified, 0 errors", `status: CLEAN`.
- `make build`: OK.

## Bug Summary
- Total bugs recorded: **0**.
- True Bugs: none. (The missing `into_raw_value` contract is a verification **coverage gap**, not a code defect.)

## Issues (highest priority first)
1. **BLOCKER — in-scope `VirtualAddress::into_raw_value` is unspecified/unverified.** Add `#[verus_verify]` to `impl Address for VirtualAddress` (or an inline `#[verus_spec]`) so the impl body is checked against `ensures result as int == self@`. Without it the module's central round-trip-identity guarantee is not established by a verified implementation; caller coverage is 3/4 and the trait obligation is unmet.
2. Minor — `make verify` (cross-module regression) was not executed in this review; `make verify-sys` passes. Run before final sign-off.
3. Non-issue (recorded to prevent re-litigation) — AST "mismatches" and spec_drift "ensures removed" are false positives from inherent-vs-trait method name collisions; no exec changed, no spec weakened. The absent trivial `inv()` is intentional for a total newtype.

## Result: **FAIL**

PASS requires every checklist item checked. The **Specification** section has
unchecked blockers: `into_raw_value` (an explicit in-scope target function) has
no verified contract on the `VirtualAddress` impl, leaving caller coverage at
3/4 and the `Address::into_raw_value` trait obligation unsatisfied. All other
dimensions (guardrails, TCB, AST integrity, `make verify-sys`, bug state) are
clean.
