# Final Comprehensive Review: arch-paging-table

Consolidated from two independent strict reviews (one per sub-agent model):
- `final_review.claude.md` (claude-opus-4.8)
- `final_review.gpt-5.3-codex.md` (gpt-5.3-codex)

Both reviewers independently re-ran every tool and reached the **same verdict: FAIL**,
with identical primary blockers. Branch: `verus-ai-prove`. In-scope functions:
`Table::write`, `TableIndex::into_raw`, `raw`, `Table::read`, `from_raw`, `pt_index`,
`TableIndex`, `pd_index`, `Table::from_address`.

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) — `find_callers_lsp.py` ran; the LSP "0 external callers" is correctly diagnosed as an x86-vs-x86_64 indexing artifact and real callers recovered by source search (`caller_analysis.md:1-60`).
- [x] Caller expectations (success + failure) documented for each pub function — `caller_analysis.md:106-178`.
- [x] Abstract resource identified — single hardware page-table page = partial map `index → Option<E>` (`caller_analysis.md:155-161`).
- [x] Pre-existing specs assessed — upstream `identity_map.spec.rs` boundary specs assessed (`caller_analysis.md:62-91`).

### View Design
- [x] Every field passes the substitution test (survives a complete rewrite) — `TableIndex@:nat`, `TableView<E>{addr,entries}` justified with rejected alternatives (`view_design.md:23-56`).
- [x] All caller-observable state represented (no missing fields) — addr + per-slot decoded entry map.
- [x] No implementation-specific fields — `base` mapped to `addr:nat`; phantom not surfaced.
- [x] inv() encodes real constraints — `TableIndex::inv = self@ < PAGE_TABLE_LENGTH` (`table.spec.rs:24-28`).
- [ ] View realized soundly without banned primitives — **FAIL.** `TableView::entries` is built from `spec_table_read → spec_table_word`, an `uninterp spec fn` (`table.spec.rs:83`), and its round-trip law leans on an `assume` axiom. The view "works" only via skill-banned primitives. (`view_design.md:330-334` vs `358-364` also contain contradictory historical statements about whether `write` carries a contents ensures.)

### Specification
- [x] Every in-scope exec function has requires/ensures — `fn_coverage.py`: 7/7 matched, 0 missing/extra.
- [ ] Caller coverage — **PARTIAL (5/7).** `write` read-after-write + single-slot frame (`caller_analysis.md:144-149`) is not in `Table::write` ensures (`table.rs:242-246`); `TableEntry` round-trip is only injected by `assume`.
- [x] View consistency — specs reference `self@`/`TableView` fields and maintain `inv()`.
- [x] No tautological ensures — none found.
- [x] No subsumed ensures — none found.
- [ ] Error paths have meaningful ensures — `read` covers `None`=invalid via `spec_table_read`; `write` has **no** postcondition at all (see caller coverage).
- [x] No assume_specification for workspace-internal code — 0 in module.
- [x] vstd searched before any assume_specification — N/A (none added in module).
- [x] Specs written for the caller — `pd_index`/`pt_index`/`into_raw`/`from_address`/`read` are directly usable.
- [x] Trait obligations satisfied — `TableEntry::raw`/`from_raw` pinned to spec projections (`table.rs:33-41`).
- [ ] Spec completeness (advisory) — `write` content transition genuinely absent (not intentional nondeterminism callers expect).
- [x] Loop invariants — N/A (no loops in module).
- [ ] No cheating on module's own functions — **FAIL.** grep: `assume`=1 (`table.proof.rs:21`), `external_body`=2 (TCB-listed), `uninterp spec fn`=3 (`table.spec.rs:61,63,83`), `trusted`=0, `admit`=0.
- [x] No specs weakened — `spec_drift.py git-diff --before verus-ai-prove`: 0 contract drift.
- [ ] Bug awareness — `bugs.md` records 0 code bugs (correct), but is itself stale/inconsistent with the code (see Bug Recording).
- [x] Cross-module regression — `make verify` exit 0; arch crate `assume=0(gate) external_body=3 admit=0`; kernel `admit=4/external_body=23` are pre-existing out-of-scope.
- [x] Verification: `make verify-arch` + `make build` — verify exit 0 (48 verified, 0 errors); build "Nothing to be done" (clean).

### Proving
- [x] No specs weakened — `spec_drift.py`: 0 drift.
- [x] Zero remaining admit() — confirmed (grep + gate `admit=0`).
- [x] Zero external_body unless listed in tcb-allowed.md — both `read`/`write` external_body are listed (`tcb-allowed.md:37-58`).
- [ ] Zero assume/assume_specification — **FAIL.** `assume(...)` at `table.proof.rs:21`.
- [x] No cfg-gated exec code — the 2 `#[cfg(verus_keep_ghost)]` are `include!` of spec/proof (non-semantic), not exec gating.
- [x] Cheating audit performed — counts/locations below.
- [x] Claimed Verus limitation has isolated reproducer — L1 (`repros/L1.rs`) isolates the uninterp-generic codec injectivity construct.
- [x] Exec rewrites minimal/semantically equivalent — no `// VERUS REWRITE` comments; AST consistent.
- [x] Cross-module regression — `make verify` exit 0.
- [ ] Verification 0 errors, 0 warnings — 0 errors, but status `CHEATING_DETECTED` due to the surviving assume/external_body.

### Cheating Elimination
- [x] Zero admit() remaining.
- [ ] Zero assume() remaining — **FAIL.** `table.proof.rs:21`.
- [x] Zero trusted functions.
- [x] Zero exec_allows_no_decreases_clause.
- [x] Zero cfg-gated exec code (only spec/proof `include!` guards present).
- [x] Zero external_body unless TCB-listed — `read`/`write` listed; no unlisted external_body.
- [x] AST consistency: zero mismatches — `✅ Consistent: 7 functions, 2 structs match.`
- [x] All exec rewrites have VERUS REWRITE comment + minimal reproducer — N/A (no exec rewrites).
- [x] Each surviving external_body confirmed in tcb-allowed.md — read, write (+ out-of-scope invlpg).
- [x] No specs weakened — spec_drift 0.
- [x] Cross-module regression — `make verify` exit 0.
- [ ] Verification 0 errors, 0 warnings — 0 errors; `CHEATING_DETECTED` status remains.
- [ ] **Additional skill-ban:** `uninterp spec fn` ×3 (`table.spec.rs:61,63,83`) — banned by `verus-constraints` ("all spec functions must have concrete definitions"); the exact "uninterp + axiom ≈ assume" anti-pattern.

### Bug Recording
- [x] bugs.md exists.
- [x] Each bug is a real code defect or correctly classified non-bug — recorded entries are Verus-limitation / deferral notes, no false "bugs".
- [ ] Each entry has What/Why/How-Verus-Helped/Severity/Suggested-Fix — entries are prose notes, not the structured format; and **entry #3 is stale**.
- [x] No external_body used to mask a code defect — the int-to-ptr boundary is a genuine Verus limitation.
- [x] Bug entries include provenance — phases noted ("Turn 1", "Proving phase").
- [ ] **Consistency:** `bugs.md:35-39` claims the lemma was converted to `external_body (empty body)` and `assume=0`; the actual code uses `assume(...)`. The discrepancy was not recorded as a finding.

## Spec Quality
The contracts that are present are clean, declarative, and caller-oriented:
`pd_index`/`pt_index` pin to declarative masking spec fns (`table.rs:101-134`);
`TableIndex::into_raw` is an identity projection plus the validated bound; the
`TableIndex` type invariant captures the exact `< PAGE_TABLE_LENGTH` guarantee callers
rely on; `from_address` faithfully preserves `base`; `read` carries a full decode
contract. **However**, two quality defects make the spec layer non-compliant:
(1) three `uninterp spec fn` (`spec_entry_raw`, `spec_entry_from_raw`, `spec_table_word`)
underpin the entire `TableView::entries` content model — banned by `verus-constraints`;
(2) `Table::write` has only a precondition and **no** contents postcondition, so the
caller-visible write semantics (and thus the read-after-write round-trip) are not
expressible from this module's contracts.

## Caller Coverage
- Covered: **5 / 7**
  1. `pd_index` exact masked extraction + range — `table.rs:101-105`.
  2. `pt_index` exact masked extraction + range — `table.rs:119-123`.
  3. `TableIndex` validated `< LEN` + `into_raw` identity — `table.spec.rs:24-28`, `table.rs:85-93`.
  4. `from_address` base identity — `table.rs:174-181`.
  5. `read` index→entry decode, `None`=invalid — `table.rs:203-213` (assumed via external_body).
- Missing / not fully covered:
  - **#6 `write` read-after-write + single-slot frame** (`caller_analysis.md:144-149`): `write` has no contents `ensures` (`table.rs:242-246`). The "Read/write round-trip" key invariant (`caller_analysis.md:170-171`) is **unprovable** from this module. The soundness reasoning for omitting an assumed postcondition is correct, but the guarantee is deferred to a permission token that does not yet exist.
  - **#7 `TableEntry` round-trip law**: established only by `assume` in `lemma_entry_roundtrip` (`table.proof.rs:21`); with #6 missing it cannot deliver the caller's read-after-write guarantee.

## Proof Completeness
- Remaining admit(): **0**.
- Remaining external_body not in tcb-allowed.md: **0** (`Table::read` `table.rs:202`, `Table::write` `table.rs:241` are both listed at `tcb-allowed.md:37-58`).
- **BLOCKER (separate dimension):** `assume(...)` at `table.proof.rs:21` closes the codec-injectivity obligation. The L1 limitation is real (two `uninterp` fns over a structureless generic `E` cannot be related in-module), but `assume` is the banned remedy — a concrete interpreted codec or a properly-declared, TCB-listed `external_body` broadcast axiom is the skill-compliant route.

## TCB Compliance
- All external_body listed in tcb-allowed.md: **NO (partial).**
  - `Table::read` (`table.rs:202`) ✅ listed (`tcb-allowed.md:37-46`).
  - `Table::write` (`table.rs:241`) ✅ listed (`tcb-allowed.md:47-58`).
  - **`lemma_entry_roundtrip`: ❌ inconsistent.** `tcb-allowed.md:59-66` (and `bugs.md:35-39`) describe it as `external_body (empty body)`, but the source uses `assume(...)` (`table.proof.rs:16/21`) with **no** `external_body` attribute. The TCB list "approves" a mechanism that is not the one in the code; `assume` is not a TCB primitive (only `assume_specification`/`external_body`/`axiom` belong on the allow-list).

## Guardrails Compliance
Raw construct-level counts (scope = 3 module files):
- admit: **0**
- assume: **1** — `table.proof.rs:21` (gate headline reports `assume=0` only via the `limitation_assume` id=L1 carve-out in `guardrails.py`; the raw source count is 1)
- external_body: **2** — `table.rs:202` (read), `table.rs:241` (write); both TCB-listed (crate-wide gate = 3, incl. out-of-scope `mod.rs:80 invlpg`)
- assume_specification: **0**
- cfg-gated exec: **0** (the 2 `#[cfg(verus_keep_ghost)]` at `table.rs:9,11` guard `include!` of spec/proof — non-semantic)
- (additional skill-banned) uninterp spec fn: **3** — `table.spec.rs:61,63,83`

Per the task hard guardrail ("ANY admit>0 OR assume>0 is a BLOCKER"): **assume=1 ⇒ BLOCKER.**
Framework carve-out note: `guardrails.py` reclassifies the L1-tagged assume as an approved
`limitation_assume`, so the automated gate passes; the `verus-constraints` skill bans
`assume` and `uninterp spec fn` unconditionally. This review follows the skills + the task's
explicit guardrail → BLOCKER.

## AST Consistency
- AST check: **PASS** — `ast_consistency.py --base-ref verus-ai-prove ... count` → `✅ Consistent: 7 functions, 2 structs match.` No `// VERUS REWRITE` / `VERUS DEVIATION` / `VERUS BUG FIX` comments present. `spec_drift.py`: 0 contract drift.

## Verification
- verus: **FAIL (strict)** — `make verify-arch` exit code **0**, 48 verified, 0 errors, but status `CHEATING_DETECTED` (`assume` + `external_body` surviving). `make build`: clean ("Nothing to be done"). `make verify` (cross-module): exit 0, no regressions.

## Bug Summary
- Total bugs recorded: **0 true code bugs** (bugs.md correctly records no code defects; its 3 entries are limitation/deferral notes).
- True Bugs: **0**.
- Documentation/process defects found by this review (not code bugs):
  1. `bugs.md:35-39` / `tcb-allowed.md:59-66` describe `lemma_entry_roundtrip` as `external_body (empty body)` and claim `assume=0`; the code uses `assume(...)` — **inconsistent, unrecorded**.
  2. Stale verified count in `bugs.md` (claims 47; current source verifies 48).
  3. `view_design.md` contains contradictory historical statements about whether `write` carries a contents ensures (`view_design.md:330-334` vs `358-364`).

## Issues (highest priority first)
1. **[BLOCKER] `assume(...)` in proof body** — `table.proof.rs:21`. Banned unconditionally by `verus-constraints`; task guardrail: any `assume>0` is a BLOCKER. Hidden in the gate headline by the id=L1 `limitation_assume` carve-out, but raw count = 1.
2. **[BLOCKER] `uninterp spec fn` ×3** — `table.spec.rs:61,63,83`. Banned by `verus-constraints`; combined with the assume axiom this is exactly the "uninterp + axiom ≈ assume" anti-pattern. The entire `TableView::entries` content model rests on them.
3. **[HIGH] Doc/code inconsistency for `lemma_entry_roundtrip`** — `tcb-allowed.md:59-66`, `bugs.md:35-39` say `external_body (empty body)` / `assume=0`; code uses `assume`. TCB list approves a mechanism not in the source; `assume` is not a TCB primitive.
4. **[MEDIUM] Missing caller invariant: read-after-write round-trip** — `Table::write` (`table.rs:241-246`) has no contents `ensures`; caller expectation #6 / key invariant (`caller_analysis.md:170-171`) is unprovable from this module's contracts. Soundly deferred, but genuinely absent.
5. **[LOW] Stale `bugs.md` (verified count, assume=0 claim) and contradictory `view_design.md` history.**

### Minimal path to PASS
- Replace the `assume` in `lemma_entry_roundtrip` with a skill-compliant mechanism: either give `spec_entry_raw`/`spec_entry_from_raw` concrete interpreted definitions so the round-trip is *proved*, or declare the lemma as an explicit, correctly-attributed `external_body`/`axiom` broadcast and make `tcb-allowed.md`/`bugs.md` accurately describe it.
- Provide concrete definitions for the three `uninterp spec fn`, or move them behind a properly TCB-listed external-bottom boundary whose recorded mechanism matches the code.
- Add a sound `write` contents transition (via the deferred page-table permission token) so caller expectation #6 is delivered.
- Reconcile `bugs.md` / `tcb-allowed.md` / `view_design.md` with the actual source.

## Result: **FAIL**

PASS requires every checklist item to pass. Unchecked items: View Design (banned primitives),
Specification (banned `uninterp`, missing `write` round-trip, cheating on own functions),
Proving (obligation closed by `assume`, non-zero cheating status), Cheating Elimination
(raw `assume>0`, `uninterp` ×3), Bug Recording (docs inconsistent with code). The automated
`make verify-arch` gate passes (0 errors) and AST/spec fidelity are clean, but under the
`verus-constraints`/`spec-design` skills and the task's explicit hard guardrail, the
`assume(...)` at `table.proof.rs:21` plus the three banned `uninterp spec fn` make this a **FAIL**.

Both independent sub-agent reviews (claude-opus-4.8 and gpt-5.3-codex) reached this same
conclusion with the same blockers.
