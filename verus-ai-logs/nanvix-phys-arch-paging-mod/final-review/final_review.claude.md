# Final Comprehensive Review: arch-paging-mod (claude-opus-4.8)

In-scope target: `src/libs/arch/src/x86/mem/paging/mod.rs::invlpg` (only).
Verification performed independently with live tool runs; the user's summary was
not trusted. Evidence (file:line + command output) is cited inline.

## Checklist  (mark [x]/[ ] with evidence)

- [x] **External-top spec correct & faithful** — `invlpg` is
  `#[verus_verify(external_body)]` (mod.rs:79) with an empty contract (no
  `requires`/`ensures`). Faithful per spec-design: inline `asm!` is an
  external-bottom hardware boundary; the TLB effect is outside Verus' memory
  model and invisible to every caller's Rust-visible state. (mod.rs:60-86)
- [x] **mod.spec.rs / mod.proof.rs intentionally empty** — both are `verus! { }`
  (mod.spec.rs:1, mod.proof.rs:1). Correct: a degenerate/empty View is the
  designed outcome (view_design.md §View Struct).
- [x] **Caller coverage 1/1** — single function; all call sites use it
  identically (flush TLB after PTE/PDE write) and none reads a result. See
  Caller Coverage below.
- [x] **admit() == 0** — `grep -rn 'admit' .` over the paging subtree → none.
- [x] **assume() == 0** — `grep -rn 'assume('` → none (the only `assume_spec…`
  hit is comment text in mod.rs:77).
- [x] **external_body all in TCB** — invlpg (mod.rs:79), table::read,
  table::write; all three listed in tcb-allowed.md (lines 52, 27, 37).
- [x] **TCB compliance** — `invlpg` listed in tcb-allowed.md line 52.
- [x] **AST consistency PASS** — `ast_consistency.py … summary`: invlpg MATCH,
  matched=1 mismatched=0.
- [x] **Verification exit 0** — `make verify-arch` exit 0; last fresh run
  "47 verified, 0 errors".
- [x] **No cfg-gated exec** — the cfg(verus_keep_ghost) lines are spec/proof
  `include!` directives, not exec code.
- [x] **No surviving true bug** — the one historic `admit()` placeholder
  (table.proof.rs) was dead spec-phase code, removed; not a bug in `invlpg`.
- [ ] **bugs.md present** — file does NOT exist. Per bug-reporting skill a
  "None" bugs file should have been written. Minor process gap (non-blocking;
  see Issues).

## Spec Quality

The faithful contract for `invlpg` is **empty** and this is correct, not a
shortcut:

- The body is a single `core::arch::asm!("invlpg ({0})", …)` (mod.rs:81-85).
  Verus does not support inline-asm expressions (verus-unsupported.md §1, with a
  minimal reproducer and the exact error: *"The verifier does not yet support …
  inline-asm expressions"*). This is a genuine external-bottom hardware
  boundary, not an avoidable proof gap.
- Per spec-design, the contract should describe WHAT a caller observes. The TLB
  is unobservable hardware state (no `PointsTo`, no value read back, no error
  path, returns `()`). There is therefore no abstract state to specify and no
  non-trivial `ensures` to prove; adding any field/postcondition would be an
  Abstraction-Leak / Over-Faithful anti-pattern (view_design.md "Rejected
  Alternatives" 1-6).
- No `requires`: the instruction is defined for any operand and is a no-op when
  no matching entry exists, so any `usize` is accepted. The ring-0 obligation is
  the `unsafe` caller's responsibility (not Verus-checkable here) — correctly
  left out of `requires`.
- The 18-line trust-boundary comment (mod.rs:69-77) clearly documents the
  boundary, the rationale, and the empty-contract justification.

**Upstream cross-check (important):** caller_analysis.md / tcb-allowed.md
describe an inherited `pub assume_specification[ ::arch::mem::paging::invlpg ]`
at `identity_map.spec.rs:151`. The live file shows that
`assume_specification` has been **removed** and replaced by a comment
(identity_map.spec.rs:151-155): *"The inherited assume_specification (empty
contract) is removed here now that the dependency module provides the identical
trusted contract."* This is the correct bottom-up outcome — the placeholder is
superseded by this module's own `external_body` contract, so there is **no
duplicated trust boundary**. The reference docs are mildly stale (they still
describe it as present) but the actual state is the desired one.

Verdict: **spec is correct, complete, and understandable.**

## Caller Coverage  (Covered 1/1, Missing: none)

All caller expectations from caller_analysis.md §"Caller Expectations" are
satisfied by the empty side-effect-only contract:

- Side-effecting on TLB only, returns `()`, no error path → satisfied (empty
  `ensures`, infallible). ✔
- Accepts any `usize`, no range check relied upon → satisfied (no `requires`). ✔
- Does not read/modify page tables, frames, or any Rust-visible state ⇒
  preserves all caller-side invariants → satisfied (empty footprint of an
  external_body that touches no Rust state). ✔
- Ring-0 obligation is caller-enforced (`unsafe`) → correctly outside the spec. ✔

Call sites confirmed (caller_analysis.md): identity_map.rs:668,
page_table.rs:210/329/385/433/498, page_directory.rs:170. Every one uses
`invlpg` identically and ignores any result. No caller needs a property the
contract omits. **Missing: none.**

## Proof Completeness

- **admit() count: 0.** `grep -rn 'admit' src/libs/arch/src/x86/mem/paging/` →
  no matches. (A historic `admit()` in table.proof.rs::lemma_entry_roundtrip was
  dead spec-phase code, removed during cheating-elimination — see Bug Summary.)
- **external_body NOT in TCB: 0.** Three `external_body` exist in the subtree —
  mod.rs:79 (invlpg), table.rs:202 (read), table.rs:241 (write) — and **all
  three are in tcb-allowed.md** (lines 52, 27, 37). The only in-scope one
  (`invlpg`) is allowlisted.

## TCB Compliance  (YES/NO)

- `src/libs/arch/src/x86/mem/paging/mod.rs::invlpg` → **YES** (tcb-allowed.md
  line 52, dedicated section "external_body introduced while speccing
  arch::x86::mem::paging (mod.rs)"). No new trust boundary introduced or
  justified by this review.

## Guardrails Compliance

Counts over the paging module subtree that the cheating gate scans (the in-scope
file mod.rs contributes only the first external_body):

- **admit: 0** — none.
- **assume: 0** — none (`grep 'assume('` → no code hits).
- **external_body: 3** — mod.rs:79 `invlpg` (in scope), table.rs:202
  `Table::read`, table.rs:241 `Table::write`. All three allowlisted in
  tcb-allowed.md. (Confirmed by `make verify-arch` cheating-detail.txt:
  mod.rs:80 invlpg, table.rs:209 read, table.rs:246 write.)
- **assume_specification: 0** — only a comment string in mod.rs:77; no
  declaration. The upstream one at identity_map.spec.rs:151 was removed.
- **cfg-gated exec: 0** — `make verify-arch` reports `cfg_gate=4`; these are the
  `#[cfg(verus_keep_ghost)] include!("…spec.rs"/"…proof.rs")` directives in
  mod.rs (lines 8,10) and table.rs (lines 9,11). They gate **spec/proof
  inclusion** (project-standard pattern), NOT exec code, so they are not a
  cfg-gated-exec deviation. **Confirmed: the cfg_gate count originates from
  these spec/proof include directives.**

## AST Consistency  (PASS)

`python3 scripts/ast_consistency.py --base-ref d2315ac…(proving start) mod.rs
summary`:

```
invlpg    MATCH
Consistent: ✅ YES (matched=1 mismatched=0 missing=0 extra=0)
```

No `// VERUS REWRITE` comments exist in scope (`grep -rn 'VERUS REWRITE'` →
none). `git diff` since proving start touches only `table.proof.rs` (a
proof-only, `#[cfg(verus_keep_ghost)]`-included file); `mod.rs`/`invlpg` exec
unchanged. **PASS.**

## Verification  (verus: PASS)

`make verify-arch` from repo root → **Exit code 0**. Cheating line:
`assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=4`.
The run was cached (no recompilation); the last fresh compile
(verus_2026-06-15_00-59-07.log) reports **"47 verified, 0 errors"**.
Full `make verify` history (fix_report.md): arch 47 verified/0 errors, kernel 76
verified/0 errors — no regressions. **PASS.**

## Bug Summary  (Total recorded 0, True Bugs 0)

- **bugs.md does not exist** for this module. No bug was recorded; none was
  discovered for `invlpg`.
- The only verification artifact resembling a bug was the historic `admit()` in
  `table.proof.rs::lemma_entry_roundtrip` — a different module's spec-phase
  placeholder. Per bug-reporting classification it is a **False Positive /
  not-a-bug**: the lemma is a domain axiom over `uninterp` codec functions,
  generic over `E` with no `TableEntry` bound, hence unprovable as written and
  also dead code (never `broadcast use`d; no caller in `src/`). It was correctly
  **removed** (not swapped for assume/external_body) and the real proof deferred
  to the `table` proving phase (verification_todo.md; fix_report.md). This is a
  legitimate deferral, not a surviving verification failure.
- No bug exists in `invlpg`: an empty external_body contract over an unobservable
  hardware instruction has no failure mode to surface.

## Issues (highest priority first)

1. **(Minor / non-blocking) bugs.md absent.** The bug-reporting skill requires
   writing "None" to the bugs file when no bugs are found. The file is missing
   entirely. Documentation hygiene only — does not affect soundness or the
   verification result.
2. **(Informational) Reference docs slightly stale.** caller_analysis.md and
   tcb-allowed.md still describe the upstream `assume_specification[…invlpg]` as
   live at identity_map.spec.rs:151, but it has been removed (now a comment).
   The actual state is the correct, stronger one (module provides the contract);
   no action required beyond noting the drift.

No BLOCKERS: admit=0, assume=0, the only in-scope external_body is allowlisted,
AST MATCH, verus exit 0.

## Result: PASS
