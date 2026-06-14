## Turn 1: Full Proving Checklist Walkthrough — phys-upool

Module in scope: `mm::phys::upool`
Target functions: `UserFrame::{new, address, leak, share, refcount, drop}`,
`Upool::{new, alloc}`.
Files: `upool.rs`, `upool.spec.rs`, `upool.proof.rs`.

### Progress
- Done (PASS/FIXED): ALL 10 checklist items — see per-item verdicts below.
- Current: none remaining.
- Remaining: none.

Verdict: **RESOLVED** — every checklist item PASS with tool evidence.

---

### Verification (commands run)

- `make verify-kernel` → exit 0. Modules verified: `mm::phys`, `mm::phys::frame`,
  `mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool`.
  Crate cheating summary: `assume=0 external_body=25 admit=0 trusted=0
  no_decreases=0 cfg_gate=10`.
- Fresh (non-cached, 2.53s/11.56s compiles) module verification log
  `verus_2026-06-15_05-17-28.log`: `mm::phys` → **19 verified, 0 errors**
  (includes `mm::phys::upool`).
- `make all-kernel` (features `microvm trace`) → exit 0, **0 warnings, 0 errors**
  (`Finished dev profile ... in 11.56s`; `kernel.elf` produced).
- `spec_drift.py check phys-upool` → exit 0, **0 contract drift**.
  Also `git-diff upool.rs --before 8d4e65c (specification END)` → 0 drift.
- Per-file cheating detector (`guardrails.detect_cheating`):
  `upool.rs`, `upool.spec.rs`, `upool.proof.rs` → `ext=0 assume=0 admit=0`.

---

### Per-item verdicts

**1. No specs weakened — PASS.**
`spec_drift.py check phys-upool` (baseline proving-start `f58e130`) and manual
`git-diff` against specification END boundary `8d4e65c` both report 0 functions
changed, 0 ensures removed, 0 requires added. The only source change in the
proving phase is removal of `#[verus_verify(external_body)]` on `UserFrame::drop`
— a strengthening (cheating removed), not a spec change. All `requires`/`ensures`
in `upool.rs` and `inv()` in `upool.spec.rs` are byte-identical to the spec phase.

**2. Zero remaining admit() — PASS.**
`admit=0` crate-wide; detector reports `admit=0` for all three upool files; grep
for `admit` in upool sources: none.

**3. Zero external_body unless TCB-listed — PASS.**
`upool.rs/.spec/.proof` → `external_body=0`. `UserFrame::drop` previously carried
`external_body` (TCB-allowed) but it was REMOVED this phase and `drop` now fully
verifies (cheating count is 25, not 26; `drop` appears in the 19 verified). The
25 crate-wide `external_body`/`external_type_spec` are all outside scope and all
enumerated in `verus-ai-logs/tcb-allowed.md`:
frame.rs (16), manager.rs (6), mod.rs (2), mod.spec.rs `ExLinkedList` (1).
No in-scope violation.

**4. Zero assume/assume_specification — PASS.**
`assume=0` crate-wide; detector `assume=0` for all upool files; no
`assume`/`assume_specification` tokens in upool sources.

**5. No cfg-gated exec code — PASS.**
upool has one counted `#[cfg(verus_keep_ghost)]` (line 39) gating a `verus! { ...
impl View for UserFrame ... }` block — pure spec, not exec. Its other cfg gates
(lines 9/11/20) gate `include!`/`use` only. Crate-wide the 10 counted gates all
wrap `verus! {}` spec blocks, `requires` clauses, or macro definitions; none gate
an exec branch, expression, or match arm.

**6. Cheating audit (counts + locations) — PASS.**
In-scope (upool): `admit=0, assume=0, external_body=0, trusted=0, no_decreases=0`;
`cfg_gate=1` (line 39, spec `verus!` block). Crate-wide (out of scope, all
TCB-listed): `external_body=24 + external_type_spec=1`, `cfg_gate=10` (all
spec/ghost). Locations confirmed via `cheating-detail.txt` and a tree-walk of
`#[cfg(verus_keep_ghost)]` sites.

**7. Claimed Verus limitation has isolated reproducer — PASS (N/A).**
No Verus limitation is currently claimed for upool. The previously-claimed
limitation (`UserFrame::drop`'s `error!`/`{:?}` Debug formatting forcing
`external_body`) was eliminated — the attribute was removed and `drop` verifies,
disproving the claim. No outstanding limitation requiring a reproducer.

**8. Exec rewrites minimal; `// VERUS REWRITE` comments — PASS.**
No `// VERUS REWRITE` comments in upool. The only exec-side change is deletion of
one attribute line; function bodies are unchanged and semantically equivalent.

**9. Cross-module regression — PASS.**
`make verify-kernel` (whole crate) verifies `mm::phys`, `frame`, `kframe`,
`manager`, `upool` with exit 0 / 0 errors; fresh log shows 19 verified, 0 errors.
No regression in sibling modules.

**10. Verification + build — PASS.**
Verus: 0 errors (19 verified, no warnings in log). Build: `make all-kernel`
exit 0, 0 warnings, 0 errors.

### Fix Request
None — all items PASS. No fixes required.
