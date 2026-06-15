# Final Independent Review — `arch::x86::mem::paging::table`

## Checklist
- [ ] Caller Analysis — `Table::write` caller expectation (read-after-write + frame) is not captured by `requires/ensures` (`caller_analysis.md:144-149` vs `table.rs:242-246`).
- [ ] View Design — document contains historical contradictory states (`view_design.md:330-334` says `write` has contents ensures; `view_design.md:358-364` says it was removed).
- [ ] Specification — external-top contract is incomplete for write effects (`table.rs:242-246` has no ensures for write transition).
- [ ] Proving — proof contains `assume(...)` (`table.proof.rs:21`), forbidden by constraints (`verus-constraints/SKILL.md:105`).
- [ ] Cheating Elimination — `cheating-detail.txt` still reports assume/external_body (`cheating-detail.txt:1-4`).
- [ ] Bug Recording — bugs log claims `assume=0` and lemma converted to `external_body` (`bugs.md:35-39`), but actual code has `assume(...)` (`table.proof.rs:21`).

## Spec Quality
- Strong/usable contracts:
  - `TableEntry::from_raw/raw` bound to spec projections (`table.rs:33-41`).
  - `TableIndex::into_raw` identity + bound (`table.rs:85-93`), with type invariant (`table.spec.rs:24-27`).
  - `pd_index`/`pt_index` specify exact masked extraction (`table.rs:101-105`, `119-123`).
  - `from_address` preserves base identity (`table.rs:174-181`).
  - `read` has explicit requires/ensures (`table.rs:203-209`).
- Gap:
  - `write` has only precondition, no postcondition (`table.rs:242-246`), so caller-visible write semantics are deferred/unproven.
- Additional strict concern:
  - `uninterp spec fn` appears 3 times (`table.spec.rs:61,63,83`), banned by constraints (`verus-constraints/SKILL.md:113`).

## Caller Coverage
**Covered 5 / 7 expectation groups from `caller_analysis.md:106-153` by requires/ensures.**

Covered:
1. `pd_index` exactness/range (`caller_analysis.md:108-114` ↔ `table.rs:101-105`).
2. `pt_index` exactness/range (`caller_analysis.md:115-119` ↔ `table.rs:119-123`).
3. `TableIndex` validated + `into_raw` identity (`caller_analysis.md:120-127` ↔ `table.spec.rs:24-27`, `table.rs:85-93`).
4. `from_address` identity (`caller_analysis.md:128-135` ↔ `table.rs:174-181`).
5. `read` decode contract (`caller_analysis.md:136-143` ↔ `table.rs:203-213`).

Missing / not fully covered:
1. `write` read-after-write + single-slot frame (`caller_analysis.md:144-149`) not in `Table::write` ensures (`table.rs:242-246`).
2. `TableEntry` round-trip expectation (`caller_analysis.md:151-153`) not established by method contracts; only injected by `assume` in proof (`table.proof.rs:21`).

## Proof Completeness
- `admit()` count: **0** (no locations).
- `external_body` declarations in module files: **2**
  - `table.rs:202` (`Table::read`)
  - `table.rs:241` (`Table::write`)
- `external_body` not in TCB allow-list: **0**
  - Both are listed (`tcb-allowed.md:37-58`).
- **Blocker:** `assume(...)` count **1** at `table.proof.rs:21`.

## TCB Compliance
**NO (strict).**

- Approved and present:
  - `Table::read` external_body (`table.rs:202`) ↔ listed (`tcb-allowed.md:37-46`).
  - `Table::write` external_body (`table.rs:241`) ↔ listed (`tcb-allowed.md:47-58`).
- Mismatch/inconsistency:
  - `tcb-allowed.md` says `lemma_entry_roundtrip` is `external_body (empty body)` (`tcb-allowed.md:59-65`), but actual code uses `assume(...)` (`table.proof.rs:21`) and has **no** `external_body` attribute.

## Guardrails Compliance
Requested hard counts (construct-level):
- `admit`: **0**
- `assume`: **1** (`table.proof.rs:21`)
- `external_body`: **2** (`table.rs:202`, `table.rs:241`)
- `assume_specification`: **0**
- cfg-gated exec (`#[cfg(not(verus_keep_ghost))]` on exec): **0**

Additional grep facts requested:
- `uninterp`: **3 declarations** (`table.spec.rs:61,63,83`)
- `trusted` attribute: **0** (word appears in comment only: `table.rs:200`)
- `exec_allows_no_decreases*`: **0**
- `#[cfg(verus_keep_ghost)]`: **2** (`table.rs:9,11`, on spec/proof includes, not exec function duplication)

Interpretation split (explicit):
1. **verus-constraints policy**: `assume(...)` is unconditionally banned (`verus-constraints/SKILL.md:105`) and `uninterp spec fn` banned (`:113`) ⇒ **FAIL/BLOCKER**.
2. **guardrails.py carve-out**: approved limitation IDs can reclassify assume (`guardrails.py:313-329`); project JSON includes `L1` (`approved-trust-boundaries.json:3-5`) ⇒ tooling may tolerate this assume.

**Strict verdict used in this review:** interpretation (1) wins per requested constraints ⇒ **BLOCKER**.

## AST Consistency
**PASS**

Command output:
- `python3 .../ast_consistency.py --base-ref verus-ai-prove .../table.rs count`
- `✅ Consistent: 7 functions, 2 structs match.`

`// VERUS REWRITE` grep in module files: **0** matches.

## Verification
**FAIL (strict final gate), error count from run: 0**

Command run:
- `make verify-arch`
- Exit code: **0**
- Verification summary text: `cached (no recompilation)` and verified/errors shown as `—` (not emitted in this run)
- Cheating line: `assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=4`
- Status line: `CHEATING_DETECTED`
- `cheating-detail.txt`:
  - `mod.rs:80 invlpg: external_body`
  - `table.proof.rs:16 lemma_entry_roundtrip: assume`
  - `table.rs:209 read: external_body`
  - `table.rs:246 write: external_body`

## Bug Summary
- Total recorded entries in `bugs.md`: **3**
  1. Verus int-to-ptr limitation (`bugs.md:7-15`)
  2. Turn 1 resolved note (`bugs.md:17-27`)
  3. Proving-phase note (`bugs.md:29-39`)
- True code bugs: **0**.
- Reconciliation outcome:
  - Entry #3 is stale/inaccurate: claims `admit=0, assume=0` and lemma converted to `external_body` (`bugs.md:35-39`), contradicted by current code (`table.proof.rs:21`) and cheating detail (`cheating-detail.txt:2`).

## Issues (highest priority first)
1. **BLOCKER:** `assume(...)` present in proof (`table.proof.rs:21`) under strict constraints banning assume.
2. **BLOCKER (strict constraints):** `uninterp spec fn` declarations present (`table.spec.rs:61,63,83`) though banned by constraints.
3. **Major spec gap:** `Table::write` lacks postcondition for caller-observable effect (`table.rs:242-246` vs caller expectation `caller_analysis.md:144-149`).
4. **Process/documentation mismatch:** `tcb-allowed.md` and `bugs.md` describe `lemma_entry_roundtrip` as external_body/assume=0, but actual code uses assume.
5. **Verification run quality:** `make verify-arch` was cached (no verified/error counts emitted), and status remains `CHEATING_DETECTED`.

## Result: **FAIL**
PASS criteria not met (checklist has blockers; strict guardrails violated).
