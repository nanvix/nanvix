# Final Independent Review — `arch-frame-number`

## Scope and inputs verified
- Reviewed only:
  - `src/libs/arch/src/x86/mem/paging/frame/number.rs`
  - `src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`
  - `src/libs/arch/src/x86/mem/paging/frame/number.proof.rs`
- Reference docs read: `caller_analysis.md`, `view_design.md`, `tcb-allowed.md`.
- `bugs.md` checked: **absent** (confirmed by filesystem error on direct read).
- `git diff verus-ai-prove -- <scope files>`: all three scope files report **NO_DIFF** (so unit tests and `NULL`/`MAX` are untouched).

---

## 1) Spec quality review (spec-design criteria)

### `FrameNumber` View + invariant
- `View` is `closed spec fn view(&self) -> int { self.0 as int }` (`number.spec.rs:13-15`).
- Bound helper is concrete/interpreted: `spec_max()` (`number.spec.rs:28-30`).
- Type invariant is explicit and strong: `0 <= self@ <= Self::spec_max()` (`number.spec.rs:35-38`).
- Verdict: **OK**. Declarative, caller-facing abstraction, no code-as-spec, no trust escape.

### `FrameNumber::from_raw_value`
- Contract (`number.rs:56-61`) states total partition:
  - if `value <= spec_max` => `Some` with exact abstract value (`result->Some_0@ == value as int`)
  - if `value > spec_max` => `None`
- This is bidirectional enough to prevent weak/spurious error behavior.
- Verdict: **OK** (no tautology, no one-sided error path).

### `FrameNumber::into_raw_value`
- Contract (`number.rs:79-83`):
  - `result as int == self@`
  - `0 <= self@ <= Self::spec_max()`
- Quality note: second clause is largely derivable from `inv()` (`number.spec.rs:35-38`), so mildly redundant/subsumed, but still useful at call sites as an explicit guarantee.
- Verdict: **OK** (minor redundancy only, non-blocking).

### Design divergence vs `view_design.md`
- `view_design.md` describes an older approach (`uninterp spec_max` + `assume_specification` tie to `MAX`).
- Shipped design uses interpreted `open spec_max()` with no `assume_specification` in scope.
- This **reduces trust boundary** and is stronger/sounder than the documented older plan.
- Verdict: **OK** (doc drift, not soundness risk).

---

## 2) Caller expectation coverage (from `caller_analysis.md`)

Expectations evaluated from `caller_analysis.md:44-79`.

| # | Caller expectation | Coverage in shipped contracts | Status |
|---|---|---|---|
| 1 | `from_raw_value`: `Some` iff `value <= MAX`, else `None` | `number.rs:58-60` split on `<= spec_max` and `> spec_max` | Covered |
| 2 | `None` path must propagate cleanly (no silent truncation) | Failure arm `> spec_max ==> None` (`number.rs:60`) + success preserves exact input (`number.rs:58-59`) | Covered |
| 3 | Round-trip identity on success | `from_raw_value` exact view (`number.rs:58-59`) + `into_raw_value` exact projection (`number.rs:81`) | Covered |
| 4 | `into_raw_value` returns exact stored frame number | `result as int == self@` (`number.rs:81`) | Covered |
| 5 | Result bounded so `<< FRAME_SHIFT` is safe (PTE/PDE callers) | bound clause (`number.rs:82`) + invariant (`number.spec.rs:35-38`) | Covered |
| 6 | `into_raw_value` total / non-failing | Signature returns `usize`; body is pure field read (`number.rs:84-87`) | Covered |
| 7 | `FrameNumber` treated as always-valid token | type invariant (`number.spec.rs:35-38`) over opaque view (`number.spec.rs:13-15`) | Covered |

- **Coverage: 7 / 7**
- Missing expectations: **0**
- Verdict: **OK**

---

## 3) Proof completeness (admit/external_body)

Checked scope files only (`number.rs`, `number.spec.rs`, `number.proof.rs`).

- `admit()` count: **0**
- `external_body` count: **0**
- `number.proof.rs` is empty (`number.proof.rs:1`).

Per rule: any `admit()>0` would be BLOCKER. Here none.

- Verdict: **OK**

---

## 4) TCB compliance

- External bodies in scope: **none**.
- Therefore every in-scope `external_body` is in `tcb-allowed.md`: **YES (vacuous)**.

- Verdict: **OK**

---

## 5) AST consistency + `VERUS REWRITE` inspection

### AST check
Command run:
- `python3 .../ast_consistency.py --base-ref verus-ai-prove src/libs/arch/src/x86/mem/paging/frame/number.rs count`
- `... summary`

Result:
- `Consistent: ✅ YES (matched=4 mismatched=0 missing=0 extra=0)`
- Structs: `FrameNumber MATCH`

### `// VERUS REWRITE`
- Search in all scope files: **0 matches**.
- So no rewrite-equivalence exceptions to audit.

- Verdict: **PASS**

---

## 6) Verification run (`make verify-arch`)

Command run:
- `cd /home/ruize/nanvix-phy-specs && make verify-arch`

Observed result:
- Verification command exit code: **0**
- Log says: `cached (no recompilation)`
- Error count in log (`verus_2026-06-15_12-18-53.log`): **0** (`error:`/`error[` matches = 0)
- Wrapper summary status: **CHEATING_DETECTED** (out-of-scope findings: paging `table.*`/`mod.rs`)

Task-6 status:
- **FAIL** (pipeline status reported cheating, despite zero verifier errors).

---

## 7) Guardrail audit (scope files only)

Exact counts + locations:

- `admit`: **0** (no locations)
- `assume(...)`: **0** (no locations)
- `external_body`: **0** (no locations)
- `assume_specification`: **0** (no locations)
- cfg-gated exec (`cfg(not(verus_keep_ghost))`): **0** (no locations)

Note:
- `number.rs` has `#[cfg(verus_keep_ghost)]` at lines `9` and `11`, but these gate spec/proof includes, not exec behavior.

Blocker policy check:
- `admit > 0`? **No**
- `assume > 0`? **No**

- Verdict: **OK**

---

## 8) Bug reconciliation

- `bugs.md` at `verus-ai-logs/nanvix-phys-arch-frame-number/bugs.md`: **does not exist** (confirmed).
- Independent code/spec/proof review found no true defect in:
  - `FrameNumber` abstraction (`view` + `inv`),
  - `from_raw_value`,
  - `into_raw_value`.

Conclusion:
- No true bug discovered that must be logged per bug-reporting policy.
- Absence of `bugs.md` is acceptable for this scope outcome.

- Verdict: **OK**

---

## 9) Spec drift check

Command run (as requested):
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff /home/ruize/nanvix-phy-specs/src/libs/arch/src/x86/mem/paging/frame/number.rs --before HEAD`

Result:
- `✅ No contract drift detected.`
- Functions with changes: 0
- Contract drift: 0

- Verdict: **OK**

---

## Blocker summary

### In-scope blockers
- None from tasks 1,2,3,4,5,7,8,9.

### Check failures
- Task 6 verification status is **FAIL** due global `CHEATING_DETECTED` in `make verify-arch` run (although verifier error count is 0 and issues are out of this module’s scope).

RESULT: FAIL
