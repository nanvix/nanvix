# Final Comprehensive Review: arch-paging-table

Module: `arch::x86::mem::paging::table`
File: `src/libs/arch/src/x86/mem/paging/table.rs` (+ `.spec.rs`, `.proof.rs`)
In-scope functions: `Table::write`, `TableIndex::into_raw`, `TableEntry::raw`,
`Table::read`, `TableEntry::from_raw`, `pt_index`, `TableIndex` (type + `new`),
`pd_index`, `Table::from_address`.

Method: two independent strict reviews were run, one per allowed model
(`claude-opus-4.8`, `gpt-5.3-codex`), saved alongside this file as
`final_review.claude.md` and `final_review.codex.md`. Both returned **PASS** with
matching guardrail counts. This document consolidates them. All heavy commands
(`make verify-arch`, `make verify`, `./z build`) were run once by the orchestrator;
read-only checks (`ast_consistency.py`, `spec_drift.py`, `fn_coverage.py`, grep)
were independently re-run by each reviewer and reproduced identical results.

Evidence (reproduced by both reviewers):
- `ast_consistency.py --base-ref 07eb0d8e4 … summary` → `Consistent: ✅ YES
  (matched=7 mismatched=0 missing=0 extra=0)`; structs `Table`, `TableIndex` MATCH.
- `spec_drift.py git-diff … --before HEAD` → `✅ No contract drift detected`.
- `fn_coverage.py` → 7 source / 7 verus exec fns, 7 matched, 0 missing, 0 extra.
- `make verify-arch` → exit 0; cheating `assume=0 external_body=3 admit=0
  trusted=0 no_decreases=0 cfg_gate=0` (the 3 are `invlpg`, `table::read`,
  `table::write` — all TCB-listed; only `read`/`write` are in this file's scope).
- `make verify` (all crates) → exit 0. `./z build` → exit 0.
- grep over the three module files: `admit` 0, `assume(` 0, `assume_specification`
  0, `trusted` 0, `exec_allows_no_decreases` 0, `VERUS REWRITE` 0; `external_body`
  attribute sites = 2 (`read` table.rs:202, `write` table.rs:241). The two
  `#[cfg(verus_keep_ghost)]` (lines 9/11) guard the `include!` of the ghost-only
  `.spec.rs`/`.proof.rs` — not cfg-gated exec code.

---

## Checklist

### Caller Analysis
- [x] All pub functions have callers searched (tool-verified). `caller_analysis.md`
  recovers real callers textually (kernel `mm/virt/identity_map.rs`, host
  `uservm/.../gva.rs`) and correctly explains the LSP false-negative (32-bit x86
  tree vs. x86_64 host indexing).
- [x] Caller expectations documented for success **and** failure paths (per-function
  "Caller Expectations" + "Key Invariants"; `None`→`InvalidArgument` recorded).
- [x] Abstract resource identified: a single hardware page-table page as a partial
  map `index → Option<E>` over caller-owned volatile memory, plus pure index
  extractors.
- [x] Pre-existing specs assessed: inherited `identity_map.spec.rs` state-free,
  `admit()`-backed `assume_specification`s judged weak/partial and superseded by
  the real contracts written here.

### View Design
- [x] Every field passes the substitution test: `TableIndex@:nat`,
  `TableView{addr:nat, entries:Map<nat,Option<E>>}` are algorithm-free; the
  mirror-the-struct alternative (`base:usize`, `PhantomData`) is correctly excluded.
- [x] All caller-observable state represented: `addr` (which page) + `entries`
  (per-slot decoded entry = the read result).
- [x] No implementation-specific fields: `base`/`PhantomData` excluded from View.
- [x] `inv()` encodes a real constraint: `TableIndex::inv` is `type_invariant
  self@ < PAGE_TABLE_LENGTH` — the validated-index guarantee every caller depends on.
- [x] Mathematical types used: `nat`, `Map<nat, Option<E>>`, `Option<E>`; addresses
  kept as `usize`/`nat` (allowed exception).

### Specification
- [x] Every in-scope exec fn has contracts (`fn_coverage.py` 7/7): `new`,
  `into_raw`, `pd_index`, `pt_index`, `from_address`, `read`, `write`,
  `raw`/`from_raw`.
- [x] Caller coverage: read `caller_analysis.md`; each expectation maps to a
  requires/ensures except the `write` read-after-write transition and the codec
  round-trip law, which are the **maximal-sound deferral** for this phase (see
  Caller Coverage — soundly motivated, fully documented, blocks no verified caller).
- [x] View consistency: `read`/`from_address` ensures reference `self@.addr`; the
  `from_address → read` chain composes.
- [x] No tautological ensures (every clause constrains result/state).
- [x] No subsumed ensures: `pd_index`/`pt_index` carry both `result@ == spec_…` and
  `result@ < LEN`; the bound needs mask reasoning, so it is a meaningful extra fact.
- [x] Error paths meaningful: `new` has explicit `None ⇒ index >= LEN`; `read`
  returns `None` exactly when `spec_entry_from_raw` rejects the word.
- [x] No `assume_specification` for workspace-internal code (0 in module).
- [x] vstd searched / appropriate math types used (`uninterp` used only to model
  genuine external-bottom state — volatile page-table memory + per-implementor
  codec — matching the project `phys_view`/`identity_map_view` convention; not
  spec-avoidance).
- [x] Specs written for the caller: `result@`, `self@.addr`, `spec_table_read`,
  `spec_pd_index`/`spec_pt_index` are directly usable in caller proofs.
- [x] Trait obligations satisfied: `TableEntry::raw`/`from_raw` pinned to
  `spec_entry_raw`/`spec_entry_from_raw`; the `E`-unbounded codec abstraction
  correctly avoids a trait↔function definitional cycle.
- [x] Spec completeness (advisory): write-transition + codec round-trip law recorded
  as proving-phase deferrals (acknowledged, not silently dropped).
- [x] Loop invariants: N/A (no loops in scope).
- [x] No cheating on the module's own functions: admit 0 / assume 0 / trusted 0 /
  no_decreases 0 / cfg_gate 0; the only `external_body` (`read`, `write`) are
  TCB-listed.
- [x] No specs weakened: `spec_drift` exit 0 vs HEAD.
- [x] Bug awareness: `bugs.md` present and reasoned (no code bugs; validated).
- [x] Cross-module regression: `make verify` (all crates) exit 0.
- [x] Verification: `make verify-arch` exit 0; `./z build` exit 0.

### Proving
- [x] No specs weakened (`spec_drift` exit 0).
- [x] Zero remaining `admit()`.
- [x] Zero `external_body` unless TCB-listed (`read`, `write` both listed).
- [x] Zero `assume`/`assume_specification`.
- [x] No cfg-gated exec (the two `#[cfg(verus_keep_ghost)]` guard ghost `include!`s).
- [x] Cheating audit counts + locations recorded (below).
- [x] Claimed Verus limitation has an isolated reproducer: `verus-unsupported.md`
  gives the minimal `usize → *const u32` cast + exact Verus error.
- [x] Exec rewrites minimal + equivalent: there are **none** (AST MATCH on all 7
  fns + 2 structs); `proof!` blocks in `pd_index`/`pt_index`/`into_raw` are
  ghost-only.
- [x] Cross-module regression green (`make verify` exit 0).
- [x] Verification 0 errors / 0 warnings (`make verify-arch` exit 0).

### Cheating Elimination
- [x] Zero `admit()`.
- [x] Zero `assume()`.
- [x] Zero trusted functions.
- [x] Zero `exec_allows_no_decreases_clause`.
- [x] Zero cfg-gated exec code.
- [x] Zero `external_body` unless TCB-listed (`read`, `write` listed).
- [x] AST consistency: zero mismatches (7 fns + 2 structs MATCH).
- [x] All exec rewrites have VERUS REWRITE comment + reproducer: vacuously true —
  there are zero exec rewrites.
- [x] Each surviving `external_body` confirmed in `tcb-allowed.md`.
- [x] No specs weakened (`spec_drift` clean).
- [x] Cross-module regression green.
- [x] Verification green (0 errors, 0 warnings).

### Bug Recording
- [x] `bugs.md` exists; states "No code bugs" with reasoning.
- [x] Each recorded item is a genuine *non-bug* (Verus int-to-ptr limitation +
  deferred abstraction) — correctly classified as a language-limitation / sound
  deferral, not a True Bug, per the bug-reporting skill.
- [x] Each (would-be) bug entry has What/Why/How-Verus-Helped/Severity/Suggested-Fix
  coverage where applicable; no surviving True Bug requiring those fields exists.
- [x] No `external_body` masking a code defect: both boundaries are genuine
  external-bottom hardware/Verus-limitation trust boundaries.
- [x] Provenance recorded: the Turn-2 unsoundness finding (pinning the pure ghost in
  `write` derives `false`) is captured across `bugs.md` / `verus-unsupported.md` /
  `view_design.md` / `tcb-allowed.md`.

---

## Spec Quality
Strong and idiomatic. `TableIndex`/`new`/`into_raw` are textbook: the validated
`self@ < LEN` guarantee is a `type_invariant` established at every construction site
and consumed by `into_raw` via `use_type_invariant`. `pd_index`/`pt_index` mirror
the exec mask `(vaddr >> shift) & (LEN-1)` exactly (shifts 22/12, mask 1023) with
the `result@ < LEN` bound discharged `by (bit_vector)`. `from_address` exposes the
only observable (`result@.addr == base`) and carries page-validity to the caller via
`unsafe`. `read` has a full decode contract `result == spec_table_read::<E>(addr,
index)` pinned to a global parameter-free page-table-memory ghost — faithfully
mirroring the `frame::instance → phys_view()` precedent; reading a pure accessor is
sound. `raw`/`from_raw` are pinned to the uninterpreted codec, `E`-unbounded to dodge
the trait cycle. The single substantive limitation is `write` (requires-only, no
contents ensures), analyzed below.

## Caller Coverage
- Covered: **11–12 / 13** (reviewers differ only on whether the codec round-trip law
  is counted as a distinct expectation; both agree the *substantive* uncovered item
  is the `write` transition).
- Missing (deferred): (12) **read-after-write round-trip** — `write` has no contents
  ensures; (13) **codec round-trip law** `from_raw(raw(e)) == Some(e)` — the
  placeholder lemma was intentionally removed rather than left as an unproven axiom.

Assessment of the key question (both reviewers concur): the `write`/round-trip gap is
an **acceptable, soundly-motivated, fully-documented deferral — NOT a blocker**:
1. The only alternative is **unsound**. `spec_table_word` is a pure `uninterp spec
   fn`; because `write` is `external_body`, any `ensures` is *assumed* at every call
   site. Pinning the pure cell to the caller's `entry` lets two distinct writes to
   the same slot assume `spec_entry_raw(e1) == spec_entry_raw(e2)`, deriving `false`
   (reproduced as a verified `assert(false)` in Turn 2).
2. The sound mechanism (a `PointsTo`-style page-table permission token threaded
   through `read`/`write`) is genuinely **out of scope**; it cascades a ghost
   parameter into out-of-scope, currently `admit()`-stubbed callers
   (`identity_map::ensure_pt`/`ensure_pte`/`identity_map_page`). This follows the
   established `phys_view()` / `identity_map_view()` "transition realized in the
   proving phase" deferral convention.
3. **No verified caller depends on it** — every real round-trip caller is itself
   `admit()`-stubbed today.
4. It is **honestly recorded** in five places (`write` comment, `bugs.md`,
   `verus-unsupported.md`, `view_design.md`, `tcb-allowed.md`).

The requires-only contract is the *maximal sound* contract for `write` in this phase.

## Proof Completeness
- Remaining `admit()`: **0** (grep clean; verify-arch `admit=0`).
- Remaining `external_body` not in `tcb-allowed.md`: **0**. The only two in this
  module — `Table::read` (table.rs:202) and `Table::write` (table.rs:241) — are both
  listed under the "introduced while speccing arch::…::table" section.
  `table.proof.rs` contains no lemmas/axioms (explanatory comments only).

## TCB Compliance
- All `external_body` listed in `tcb-allowed.md`: **YES**. `read` and `write` are both
  listed, each justified by the genuine `usize → *const/*mut T` Verus limitation with
  a minimal reproducer + exact error. No new trust boundary introduced. (The third
  arch `external_body`, `invlpg`, lives in sibling `mod.rs`, out of this file's scope,
  and is also listed.)

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **2** (`read`, `write`; both
  TCB-listed), assume_specification: **0**, cfg-gated exec: **0**.
- Also: trusted **0**, exec_allows_no_decreases **0**.
- (Crate-wide `make verify-arch` reports `external_body=3` due to out-of-scope
  `invlpg`; still TCB-listed.)

## AST Consistency
- AST check: **PASS** — `matched=7 mismatched=0 missing=0 extra=0`; structs
  `Table`/`TableIndex` MATCH; 0 `// VERUS REWRITE`. Exec code is semantically
  identical to baseline `07eb0d8e4`; all added material is ghost.

## Verification
- verus: **PASS** — `make verify-arch` exit 0, arch crate verified, 0 errors,
  0 warnings. `make verify` (all crates) exit 0; `./z build` exit 0. `spec_drift`
  vs HEAD: 0 contract drift. (Pre-existing out-of-scope kernel-crate
  admit/external_body/cfg_gate are unrelated to this module.)

## Bug Summary
- Total bugs recorded: **0 code bugs** (`bugs.md` records two *non-bug* notes: the
  Verus int-to-ptr limitation and the deferred write-transition abstraction).
- True Bugs: **0**. The in-scope exec bodies are correct (index masks, shift
  offsets, decode-on-read, encode-on-write all match the hardware contract).

## Issues (highest priority first)
1. **(Advisory, non-blocking)** `write` read-after-write round-trip + codec law
   deferred to the proving phase (needs an out-of-scope page-table permission token;
   the unsound shortcut derives `false`). Not a blocker.
2. **(Minor doc staleness)** Stale `lemma_entry_roundtrip` references: the lemma was
   correctly removed, but `table.rs:233` and `tcb-allowed.md:44` still cite it.
   Recommend rewording to "a future codec round-trip law would derive …". Cosmetic.
3. **(Minor doc staleness)** `bugs.md` Turn-1 "resolved" note no longer fully
   reflects the Turn-2 correction (write contents ensures removed). Documentation
   drift only.
4. **(Minor design divergence)** `view_design.md` proposed a `TableView::inv`
   (domain + `addr % PAGE_SIZE == 0`); as built the domain is folded into `view()`
   and page-alignment is an unenforced caller-carried `unsafe` obligation. Acceptable
   for a non-owning handle; worth a one-line reconciliation in the design note.

Items 2–4 are documentation/cosmetic and affect no verification artifact.

---

## Result: PASS

Every hard guardrail is clean (admit 0, assume 0, assume_specification 0, trusted 0,
exec_allows_no_decreases 0, cfg-gated exec 0; both `external_body` are TCB-listed).
AST consistency is an exact MATCH (no exec mutation, no `// VERUS REWRITE`), spec
drift is zero, and `make verify-arch` / full `make verify` / `./z build` all pass
with zero errors and zero warnings. In-scope success and error contracts are correct,
non-tautological, caller-usable, and implementation-independent. The single
substantive gap — `write` lacking a contents `ensures` — is the **maximal sound**
contract for this phase: the only alternative is a demonstrably unsound assumed
postcondition, and the sound realization needs out-of-scope proving-phase machinery,
following the project's established deferral convention. It blocks no verified caller
and is transparently documented. Both independent model reviews
(`final_review.claude.md`, `final_review.codex.md`) reached PASS with identical
guardrail counts. No checklist item is left unsatisfied.
