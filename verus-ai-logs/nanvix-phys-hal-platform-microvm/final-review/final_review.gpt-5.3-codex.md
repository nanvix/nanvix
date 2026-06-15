# Final Independent Review — `hal::platform::microvm::gva_to_gpa`

## Scope and Method
- **In scope only:** `gva_to_gpa` in `src/kernel/src/hal/platform/microvm/mod.rs`.
- **Out of scope:** all other functions in `hal::platform::microvm`.
- I verified claims with repository/tool evidence only.

## Skill-doc routing check (requested by prompt)
Command:
```bash
find .github/skills -name SKILL.md | sort | grep -E 'spec-design|verus-constraints|ast-consistency|bug-reporting|spec-drift-check|spec-completeness' || true
```
Output: *(no matches)*

The specifically requested skill docs are not present under `.github/skills` in this checkout.

---

## 1) Spec quality (`#[verus_spec]` + `spec_gva_to_gpa`) — **PASS**

### Evidence
Definition/spec/caller lines:
```bash
grep -n "gva_to_gpa" -n \
  src/kernel/src/hal/platform/microvm/mod.rs \
  src/kernel/src/hal/platform/microvm/mod.spec.rs \
  src/kernel/src/mm/phys/mod.rs
```
```text
src/kernel/src/hal/platform/microvm/mod.rs:427:        result as int == spec_gva_to_gpa(gva as int),
src/kernel/src/hal/platform/microvm/mod.rs:430:pub fn gva_to_gpa(gva: usize) -> usize {
src/kernel/src/hal/platform/microvm/mod.spec.rs:14:pub open spec fn spec_gva_to_gpa(gva: int) -> int {
src/kernel/src/mm/phys/mod.rs:114:            let mmio_addr: usize = crate::hal::platform::gva_to_gpa(start);
```

Relevant source excerpts:
- `mod.rs:425-428`: ensures `result as int == spec_gva_to_gpa(gva as int)`
- `mod.rs:430-431`: exec body is identity (`gva`)
- `mod.spec.rs:14-15`: `pub open spec fn spec_gva_to_gpa(gva: int) -> int { gva }`

### Assessment
- **Correctness:** matches exec body and MicroVM identity mapping contract.
- **Completeness for this function:** single postcondition captures exact behavior.
- **Understandability:** named spec function + comments in `mod.spec.rs` are clear.
- **`open` correctness:** reasonable; callers can unfold identity when needed.
- **`int` domain:** appropriate for mathematical reasoning; avoids machine-overflow artifacts in specs.
- **Non-tautological:** not vacuous; it constrains output to input identity.
- **Non-subsumed:** only one ensures clause; not redundant with another clause.
- **Identity contract appropriateness:** for MicroVM, yes; this is exactly what caller relies on.

---

## 2) Caller coverage vs caller_analysis — **PASS**

### Evidence
Caller use:
- `src/kernel/src/mm/phys/mod.rs:114` uses result as MMIO physical address candidate.
- `caller_analysis.md:67-88, 110-117` lists expectations: total/infallible, deterministic, frame correspondence, identity on MicroVM.

Spec facts available:
- No `requires` on `gva_to_gpa` (`mod.rs:425-428`) ⇒ callable for any `usize`.
- Ensures identity via open spec (`mod.rs:427`, `mod.spec.rs:14-15`).

### Mapping
- **Totality/infallibility:** captured (no precondition, plain `usize -> usize`, no failure arm).
- **Determinism/purity:** derivable (pure function contract + identity map).
- **Frame correspondence/order stability:** derivable from `result == gva`.
- **Identity:** directly captured.
- **Failure expectations:** N/A (caller analysis explicitly says none for this function).

No uncovered caller expectation found.

---

## 3) Proof completeness (`admit`, `external_body`) in microvm module files — **PASS**

### Evidence
Scoped files:
```bash
ls -1 src/kernel/src/hal/platform/microvm/*.rs
```
```text
mod.proof.rs
mod.rs
mod.spec.rs
pvclock.rs
start.rs
start16.rs
```

Counts (scoped):
```bash
grep -R -n -E 'admit[[:space:]]*![[:space:]]*\(|(^|[^[:alnum:]_])admit[[:space:]]*\(' src/kernel/src/hal/platform/microvm/*.rs | wc -l
# 0

grep -R -n 'external_body' src/kernel/src/hal/platform/microvm/*.rs | wc -l
# 0
```

- `admit() = 0` (**blocker threshold not hit**)
- `external_body = 0`

---

## 4) TCB compliance (`external_body` vs allowed list) — **PASS**

### Evidence
- Scoped `external_body` count is 0 (above).
- `tcb-allowed.md` has no `microvm` entries (search returned none):
```bash
grep -n "microvm" verus-ai-logs/tcb-allowed.md || true
```

### Assessment
No microvm `external_body` items exist, so **no TCB violations**.

---

## 5) AST consistency + `VERUS REWRITE` check — **PASS**

### Evidence
Requested command (absolute path) failed due script path handling:
```bash
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py /home/ruize/nanvix-phy-specs/src/kernel/src/hal/platform/microvm/mod.rs
```
```text
ERROR: Cannot get original from git: fatal: path '/home/ruize/.../mod.rs' exists on disk, but not in '<sha>'
```

Equivalent repo-relative run succeeded:
```bash
cd /home/ruize/nanvix-phy-specs
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/kernel/src/hal/platform/microvm/mod.rs
```
```text
✅ All exec functions consistent.
Functions matched: 28/28
Functions mismatched: 0
Consistent: YES
```

`VERUS REWRITE` markers in scoped module files:
```bash
grep -R -n 'VERUS REWRITE' src/kernel/src/hal/platform/microvm/*.rs | wc -l
# 0
```

No AST mismatch; no rewrite markers requiring manual equivalence audit.

---

## 6) Verification run (`make verify-kernel`) — **PASS**

### Evidence
Command:
```bash
cd /home/ruize/nanvix-phy-specs && make verify-kernel
```
Key output:
```text
Exit code : 0
status: CHEATING_DETECTED
```

Assessment:
- Requirement in prompt was to confirm `make verify-kernel` exit status: **0 confirmed**.
- Kernel-wide cheating summary exists, but scoped microvm guardrail counts are separately audited below.

---

## 7) Guardrail counts (scoped to microvm module files only) — **PASS**

### Evidence
Scoped files and exact counts:
```bash
cd /home/ruize/nanvix-phy-specs
ls -1 src/kernel/src/hal/platform/microvm/*.rs

grep -R -n -E 'admit[[:space:]]*![[:space:]]*\(|(^|[^[:alnum:]_])admit[[:space:]]*\(' src/kernel/src/hal/platform/microvm/*.rs | wc -l
grep -R -n -E 'assume[[:space:]]*![[:space:]]*\(|(^|[^[:alnum:]_])assume[[:space:]]*\(' src/kernel/src/hal/platform/microvm/*.rs | wc -l
grep -R -n 'external_body' src/kernel/src/hal/platform/microvm/*.rs | wc -l
grep -R -n 'assume_specification' src/kernel/src/hal/platform/microvm/*.rs | wc -l
grep -Pzo '(?ms)#\[cfg\([^\n]*\)\]\n(?:\s*#\[[^\n]*\]\n)*\s*(?:pub\s+)?(?:unsafe\s+)?fn\s+\w+' src/kernel/src/hal/platform/microvm/*.rs | awk -v RS='\0' 'END{print NR}'
```
Output summary:
- `admit = 0`
- `assume = 0`
- `external_body = 0`
- `assume_specification = 0`
- `cfg-gated exec = 8`

Blocker policy check:
- `admit > 0`? **No**
- `assume > 0`? **No**

---

## 8) Bug reconciliation (`bugs.md`) — **PASS**

### Evidence
```bash
[ -f verus-ai-logs/nanvix-phys-hal-platform-microvm/bugs.md ] && echo EXISTS || echo MISSING
```
```text
MISSING
```

Assessment:
- No existing bug ledger to reconcile.
- Current in-scope function is identity in exec (`mod.rs:430-431`) and identity in spec (`mod.spec.rs:14-15`), with matching ensures (`mod.rs:427`); no unrecorded defect found for `gva_to_gpa`.

---

## 9) Spec drift (`spec_drift.py ... --before HEAD`) — **PASS**

### Evidence
Command:
```bash
cd /home/ruize/nanvix-phy-specs
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff src/kernel/src/hal/platform/microvm/mod.rs --before HEAD
```
Output:
```text
Functions with changes: 0
Contract drift: 0
Ensures removed: 0
Requires added: 0
✅ No contract drift detected.
```

No weakening detected.

---

## Exact cheating counts (requested)

### Scoped to `hal/platform/microvm/*.rs` (authoritative for this review)
- admit=0
- assume=0
- external_body=0
- assume_specification=0
- cfg-gated exec=8

### Kernel-wide (from `make verify-kernel`, informational only)
- assume=0 external_body=19 admit=12 trusted=0 no_decreases=0 cfg_gate=19

---

## Final Verdict — **PASS**

No blockers found in scope for `gva_to_gpa`.

### Prioritized issues
1. **None (blocker-level) in scope.**
2. Informational: requested skill documents (`spec-design`, `verus-constraints`, `ast-consistency`, `bug-reporting`, `spec-drift-check`, `spec-completeness`) are not present under `.github/skills` in this checkout.
