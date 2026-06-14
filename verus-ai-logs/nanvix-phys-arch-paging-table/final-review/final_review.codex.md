# Final Comprehensive Review: arch-paging-table (gpt-5.3-codex)

## Checklist
### Caller Analysis
- [x] All pub functions have callers searched (tool-verified, not manual claim) (`caller_analysis.md` + repo search evidence in file).
- [x] Caller expectations (success + failure) documented for each pub function (`caller_analysis.md`, sections “Caller Expectations” / “Trait Obligations”).
- [x] Abstract resource identified (page-table page as `index -> Option<E>` over volatile memory).
- [x] Pre-existing specs assessed (upstream placeholder assumptions in `identity_map.spec.rs` documented in `caller_analysis.md`).

### View Design
- [x] Every field passes the substitution test (TableIndex nat view; TableView `{addr, entries}` remains algorithm-independent).
- [x] All caller-observable state represented (validated index + decoded slot map).
- [x] No implementation-specific fields in View (`PhantomData`/pointer mechanics excluded from View).
- [x] `inv()` encodes real constraints (non-trivial `TableIndex::inv(): self@ < PAGE_TABLE_LENGTH`).
- [x] Mathematical types used (`nat`, `Map<nat, Option<E>>`, `Option<E>`; address kept as `usize`/`nat`).

### Specification
- [x] Every in-scope exec function has contracts (`fn_coverage.py`: 7/7 matched); `write` intentionally has requires-only contract with documented deferred transition.
- [x] Caller coverage reviewed against `caller_analysis.md` (one write-transition expectation explicitly deferred; see Caller Coverage section).
- [x] View consistency checked against `view_design.md` (contracts reference `self@`, `index@`, `spec_*` view-level functions).
- [x] No tautological ensures detected.
- [x] No harmful subsumed ensures detected (minor redundancy like explicit bound restatements is non-blocking).
- [x] Error-path semantics present where applicable (`TableIndex::new` Some/None split; `read`/`from_raw` use `Option` decode contract).
- [x] No `assume_specification` for workspace-internal code in module files.
- [x] vstd search before `assume_specification`: N/A (none used).
- [x] Specs are caller-oriented (index extractors exact; read decode exact; write deferral explicitly documented as trust-boundary design).
- [x] Trait obligations reviewed (`TableEntry::raw`/`from_raw` are bound to spec codec functions).
- [x] Spec completeness assessed (known deferred write-transition tokenization documented; no hidden weakening).
- [x] Loop invariants: N/A (no loops in in-scope functions).
- [x] No cheating on module’s own functions beyond approved trust boundaries (counts below).
- [x] No specs weakened (`spec_drift.py git-diff ... --before HEAD`: 0 drift).
- [x] Bug awareness checked against `bugs.md` and current code/comments.
- [x] Cross-module regression evidence available from pre-run `make verify` (exit 0, provided in prompt/pipeline logs).
- [x] Verification evidence available from pre-run `make verify-arch` + `./z build` (both exit 0).

### Proving
- [x] No specs weakened (`spec_drift.py`: clean).
- [x] Zero remaining `admit()` in `table.rs`/`table.spec.rs`/`table.proof.rs`.
- [x] Surviving `external_body` are only TCB-listed ones (`Table::read`, `Table::write`).
- [x] Zero `assume`/`assume_specification` in scope.
- [x] No cfg-gated exec divergence (`cfg(verus_keep_ghost)` includes only spec/proof files).
- [x] Cheating audit done with exact counts/locations.
- [x] Claimed Verus limitation has isolated reproducer (`verus-unsupported.md`: usize->pointer cast minimal repro + exact error).
- [x] Exec rewrites checked (`VERUS REWRITE` absent; AST-consistency PASS).
- [x] Cross-module regression evidence from pre-run `make verify` exit 0.
- [x] Verification evidence from pre-run `make verify-arch`/`./z build` exit 0.

### Cheating Elimination
- [x] Zero `admit()` remaining.
- [x] Zero `assume()` remaining.
- [x] Zero trusted functions.
- [x] Zero `exec_allows_no_decreases_clause`.
- [x] Zero cfg-gated exec code (semantic divergence).
- [x] No unlisted `external_body`.
- [x] AST consistency: zero mismatches (7/7 fns, 2/2 structs MATCH).
- [x] Exec rewrite policy satisfied (no rewrites present).
- [x] Each surviving `external_body` confirmed in `tcb-allowed.md`.
- [x] No spec weakening (`spec_drift.py`: clean).
- [x] Cross-module regression evidence present (`make verify` pre-run PASS).
- [x] Verification evidence present (`make verify-arch` pre-run PASS).

### Bug Recording
- [x] `bugs.md` exists.
- [x] Entries reconciled as limitations/status notes; no unresolved true code defect in-scope.
- [x] No surviving true-bug entry missing mandatory fields (none classified as true bug).
- [x] No `external_body` used to mask an in-scope code defect.
- [x] Provenance/context present (turn/phase notes in logs + `bugs.md`/`view_design.md`).

## Spec Quality
Contracts are clear and mostly caller-usable: `TableIndex::{new,into_raw}`, `pd_index`, `pt_index`, `from_address`, and `read` are precise. `read`’s decode contract (`result == spec_table_read(addr,index)`) is strong and aligns with `TableEntry::from_raw`. `Table::write` intentionally omits contents ensures; this avoids known unsoundness for assumed `external_body` postconditions over pure ghost memory and is explicitly documented in code + `tcb-allowed.md` + `verus-unsupported.md`. `TableIndex::inv()` is non-trivial and load-bearing.

## Caller Coverage
- Covered: 12 / 13
- Missing: [
  "`Table::write` read-after-write / single-slot transition guarantee is not encoded as an ensures (documented proving-phase deferral via permission token)."
]

Assessment of the critical point: this is an **acceptable documented deferral (not a current BLOCKER)** given the demonstrated unsoundness of assuming a pure-cell write postcondition on `external_body`, and explicit TCB/limitation documentation.

## Proof Completeness
- Remaining admit(): 0 []
- Remaining external_body not in tcb-allowed.md: 0

## TCB Compliance
- All external_body listed in tcb-allowed.md: YES

Evidence: in-scope `#[verus_verify(external_body)]` only at `table.rs` (read/write), both listed under `arch::x86::mem::paging::table` in `verus-ai-logs/tcb-allowed.md`.

## Guardrails Compliance
- admit: 0, assume: 0, external_body: 2, assume_specification: 0, cfg-gated exec: 0

(verify-arch crate-wide log shows `external_body=3` because of out-of-scope `paging::invlpg`; still TCB-listed.)

## AST Consistency
- AST check: PASS

Independent checks run:
- `ast_consistency.py --base-ref 07eb0d8e4 ... summary` => 7 MATCH / 0 mismatch, 2 structs MATCH.
- `ast_consistency.py ... report -o ...` => 0 diffs.
- `VERUS REWRITE` scan => none.

## Verification
- verus: PASS

Evidence used:
- Pre-run `make verify-arch`: exit 0, verified; cheating summary `assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- `verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt` confirms the 3 external bodies are `invlpg`, `table::read`, `table::write`.
- Pre-run `make verify` and `./z build`: exit 0 (provided in prompt/pipeline logs).

## Bug Summary
- Total bugs recorded: 2
- True Bugs: 0 []

Reconciliation:
1. Verus limitation entry (int-to-pointer cast) is valid.
2. “Resolved Turn 1 — read/write now carry full contracts” is partially stale after Turn 2 correction (write contents ensures removed). This is documentation drift, not an in-scope runtime code defect.

## Issues (highest priority first)
1. Documentation drift in `bugs.md`: Turn-1 resolution text no longer fully reflects current write-contract status.
2. Caller-visible write transition remains deferred (documented design tradeoff; not a blocker under current TCB/limitations).

## Result: PASS
