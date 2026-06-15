# Final Comprehensive Review: sys-virt-address

> Consolidated from two independent reviews (`final_review.claude.md` —
> claude-opus-4.8; `final_review.codex.md` — gpt-5.3-codex) and
> orchestrator-verified evidence. Both reviewers independently reached **FAIL**
> on the same root blocker. Review-only: no source/spec/proof code was modified.
>
> In-scope target functions (ONLY): `VirtualAddress::into_raw_value`,
> `VirtualAddress::from_raw_value` (inherent), `VirtualAddress::new`, and the type
> `VirtualAddress` (struct + `View`).

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim)
- [x] Caller expectations (success + failure) documented for each pub function
- [x] Abstract resource identified
- [x] Pre-existing specs assessed (if any exist from upstream verification)

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite)
- [x] All caller-observable state represented (no missing fields)
- [x] No implementation-specific fields (only caller-observable state)
- [~] inv() encodes real constraints (not trivially true) — design specifies `inv()==true` (a total newtype has no semantic invariant); however **no `inv()` is defined in source**, a doc/source divergence (harmless: no in-scope spec consumes it)
- [x] Mathematical types used (`type V = int`; address abstraction)

### Specification
- [ ] Every in-scope exec function has requires/ensures — **FAIL**: `into_raw_value` has NO `#[verus_spec]` and its `impl Address` block has NO `#[verus_verify]`
- [ ] Caller coverage: each caller expectation has corresponding requires/ensures — **FAIL**: round-trip inverse via `into_raw_value` is unmet
- [x] View consistency: specs reference View (`result@`/`self@`) and the total type maintains `inv()` trivially
- [x] No tautological ensures
- [x] No subsumed ensures
- [x] Error paths have meaningful ensures (all in-scope fns are total/infallible — no error arm needed)
- [x] No assume_specification for workspace-internal code (none present)
- [x] vstd searched before any assume_specification (none used)
- [ ] Specs written for the caller (usable directly in caller proofs) — **FAIL** for `into_raw_value` (no contract to use)
- [ ] Trait obligations satisfied (specs match trait-level semantic contracts) — **FAIL**: trait `Address` declares `into_raw_value` ensures `result as int == self@` and `from_raw_value` Ok/Err arms, but the impl block is not under `#[verus_verify]`, so the impl is never checked against them
- [ ] Spec completeness (advisory) — round-trip inverse property incompletely established
- [x] Loop invariants — N/A (no loops)
- [x] No cheating on module's own functions: `admit=0`, `assume=0`, `external_body=0`, `trusted=0`
- [x] No specs weakened (`spec_drift.py`: only false-positive from duplicate `from_raw_value` name; working tree == HEAD)
- [x] Bug awareness: no fundamentally incorrect code found
- [x] Cross-module regression: verified modules still pass (verus-sys exit 0)
- [~] Verification: `make verify-sys` exit 0, `2 verified, 0 errors`; build PASS — but only 2 in-scope fns verified (`into_raw_value` not among them)

### Proving
- [x] No specs weakened (spec_drift false-positive only)
- [x] Zero remaining admit()
- [x] Zero external_body (none in scope; TCB hard rule trivially satisfied)
- [x] Zero assume/assume_specification
- [x] No cfg-gated exec code in scope (only `#[cfg(verus_keep_ghost)] include!` spec/proof mechanism)
- [x] Cheating audit: admit=0, external_body=0, assume=0, cfg-gated-exec=0 (in-scope files)
- [x] No claimed Verus limitation (none claimed; no `// VERUS REWRITE`)
- [x] Exec rewrites minimal/equivalent — none performed; in-scope exec byte-identical to baseline
- [x] Cross-module regression: verus-sys exit 0
- [~] Verification: `make verify-sys` exit 0 / `./z build` OK — 0 errors, but `into_raw_value` unverified

### Cheating Elimination
- [x] Zero admit() remaining
- [x] Zero assume() remaining
- [x] Zero trusted functions
- [x] Zero exec_allows_no_decreases_clause
- [x] Zero cfg-gated exec code in scope
- [x] Zero external_body (TCB hard rule satisfied: nothing to list)
- [ ] AST consistency: zero mismatches — **raw tool FAIL** (`count` → mismatched/extra). Substance: in-scope exec is byte-identical (per-function `diff --name` = MATCH); mismatches are a false positive (duplicate `from_raw_value` name collision) plus an out-of-scope EXTRA (`clone_address`). Per the strict "any MISMATCH is a blocker" rule this item is **not checked**.
- [x] All exec rewrites have VERUS REWRITE comment — N/A (no rewrites)
- [x] Each surviving external_body in TCB — N/A (zero external_body)
- [x] No specs weakened (spec_drift false-positive only)
- [x] Cross-module regression passes
- [~] Verification: 0 errors / build OK, with `into_raw_value` coverage caveat

### Bug Recording
- [ ] bugs.md exists if bugs were found — **process gap**: `bugs.md` absent; per bug-reporting skill it should record `None`
- [x] Each bug is a real code defect — N/A (no code defects found)
- [x] Each bug entry has What/Why/HowVerusHelped/Severity/SuggestedFix — N/A
- [x] No external_body used to mask a code defect
- [x] Bug entries include provenance — N/A

## Spec Quality
`VirtualAddress::new` (`ensures result@ == value as int`) and inherent
`VirtualAddress::from_raw_value` (`ensures result@ == raw_addr as int`) are
correct, minimal, declarative, caller-usable, and match the `int` View design.
Constructor equivalence (`new(x)@ == from_raw_value(x)@`) follows. The `View for
VirtualAddress` (`closed spec fn view = self.0 as int`) is the correct, minimal
abstraction shared with sibling address types.

**Major defect — `into_raw_value`.** This in-scope `Address` trait method has **no
`#[verus_spec]`**, and its `impl Address for VirtualAddress` block (virt.rs:167)
has **no `#[verus_verify]`** — so the entire trait impl (including `into_raw_value`
and the trait `from_raw_value`) is invisible to Verus: not merely unspecified but
unchecked. The trait `Address` itself does declare the contract
(`ensures result as int == self@` for `into_raw_value`; Ok/Err arms for
`from_raw_value`), but without `#[verus_verify]` on the impl those obligations are
never discharged for `VirtualAddress`. The caller-mandated round-trip inverse is
therefore unestablished.

## Caller Coverage
- Covered: **2 / 3** in-scope functions (`new`, inherent `from_raw_value`); type
  `VirtualAddress` View covered separately.
- Missing:
  - `Address::into_raw_value` — 3 call sites (`mm/mmio.rs:67`
    `u32::try_from(base.into_raw_value())`; `pm/sync.rs:37` and `:65`
    `From<MutexAddress>`/`From<ConditionAddress> for usize`) rely on
    `into_raw_value() == self@`. No ensures exists → expectation unmet.
  - Consequently the central **round-trip identity**
    (`new(x).into_raw_value() == x`, `from_raw_value(x).into_raw_value() == x`)
    is only half-proven; callers cannot derive it.

## Proof Completeness
- Remaining admit(): **0** (none in virt.rs / virt.spec.rs / virt.proof.rs).
- Remaining external_body not in tcb-allowed.md: **0** (zero external_body in scope).
- Note: this is *not* a proof-soundness failure but a **coverage** failure — an
  in-scope function (`into_raw_value`) is left entirely unverified.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **YES** (vacuously — zero
  `external_body` / `assume_specification` / `axiom` in the in-scope files; no new
  trust boundary introduced).

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **0**, assume_specification: **0**,
  cfg-gated exec: **0** (across virt.rs / virt.spec.rs / virt.proof.rs).
- Note: `make verify-sys` reports a crate-level `cfg_gate=1` → traced to the
  out-of-scope `src/libs/sys/src/sys/mm/alignment.rs:151`
  (`#[cfg(verus_keep_ghost)] verus! { ... }`, the standard verification-material
  guard, not exec-hiding). virt.rs contributes 0; `cheating-detail.txt` empty.

## AST Consistency
- AST check: **FAIL** (raw tool reports mismatched/extra). Substance: in-scope
  exec is byte-identical to baseline (per-function diff = MATCH). The signals are
  a false positive (duplicate `from_raw_value` name collision in the name-keyed
  summary) plus a genuine but **out-of-scope** EXTRA, `clone_address` (added by
  sibling commit `40a4c4b60` as a now-required trait method). No `// VERUS
  REWRITE` comments exist. Per the strict "any MISMATCH is a blocker" rule this is
  reported as FAIL.

## Verification
- verus: `make verify-sys` exit **0**, `2 verified, 0 errors`; `./z build` →
  `[OK] Build complete.` → **PASS for the two verified functions**, but the
  in-scope `into_raw_value` is **not** among the verified set (its impl block is
  not under `#[verus_verify]`). Overall verification objective therefore **not
  met**.

## Bug Summary
- Total bugs recorded: **0** (`bugs.md` absent — should record `None`).
- True Bugs: **0** — the in-scope exec is a correct, total `usize` newtype wrapper
  with no overflow/cast/off-by-one hazard. The `into_raw_value` gap is a
  specification/coverage incompleteness, not a code defect. No bug was masked by
  `external_body`.

## Issues (highest priority first)
1. **BLOCKER — `into_raw_value` unspecified AND unverified.** In-scope
   `Address::into_raw_value` (virt.rs:253) has no `#[verus_spec]`, and its
   `impl Address for VirtualAddress` (virt.rs:167) has no `#[verus_verify]`, so
   Verus never checks it against the trait contract. The caller-relied inverse
   (`result as int == self@`, mandated by caller_analysis Key-Invariant #1 and
   view_design §4) is absent; 3 call sites cannot discharge round-trip identity.
   Desk-reject-level coverage gap. **Fix:** add `#[verus_verify]` to the
   `impl Address` block and `#[verus_spec(result => ensures result as int ==
   self@)]` to `into_raw_value` (and the Ok/Err ensures to the trait
   `from_raw_value`), then re-verify.
2. **MAJOR — Caller coverage incomplete (2/3 in-scope fns).** Round-trip /
   inverse / purity guarantees for `into_raw_value` not formalized.
3. **MEDIUM — `clone_address` extra uncontracted exec vs baseline.** Out of
   in-scope set; a required trait method from a sibling kernel commit; flagged
   `EXTRA … REMOVE` by fn_coverage. Confirm intent and eventually specify.
4. **LOW — Process: `bugs.md` missing.** Create with `None` per bug-reporting skill.
5. **INFO — Tooling false positives.** AST `mismatched` and spec_drift "ensures
   removed" stem from the duplicate `from_raw_value` name; not real changes
   (working tree == HEAD). `inv()` not defined though view_design specifies
   `inv()==true` (harmless doc/source divergence). Crate `cfg_gate=1` is the
   benign out-of-scope `alignment.rs` ghost guard.

## Result: FAIL

Both independent reviewers (claude-opus-4.8 and gpt-5.3-codex) reached FAIL on the
same root cause. `VirtualAddress::new` and inherent `VirtualAddress::from_raw_value`
are correctly and cleanly specified and verified (no admit/assume/external_body),
and the build passes — but the explicitly in-scope `VirtualAddress::into_raw_value`
carries no contract and is not even verified (its `impl Address` block lacks
`#[verus_verify]`), leaving the abstraction's central round-trip/inverse property —
on which 3 caller sites depend — unprovable. Multiple checklist items are
therefore unchecked. Promote to PASS only after `into_raw_value` (and ideally the
trait `from_raw_value`) are annotated, verified, and `bugs.md` recorded.
