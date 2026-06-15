# Independent Final Review — `arch::x86::mem::paging::table`

- Reviewer: independent strict review (Claude)
- Date: 2026-06-15
- Branch: verus-ai-prove
- Scope files:
  - `src/libs/arch/src/x86/mem/paging/table.rs`
  - `src/libs/arch/src/x86/mem/paging/table.spec.rs`
  - `src/libs/arch/src/x86/mem/paging/table.proof.rs`
- In-scope functions: `Table::write`, `TableIndex::into_raw`, `raw`, `Table::read`,
  `from_raw`, `pt_index`, `TableIndex`, `pd_index`, `Table::from_address`

**Verdict up front: FAIL.** The implementation verifies cleanly and the automated
cheating gate passes (`assume=0 external_body=3 admit=0`), but the *raw source*
contains an `assume(...)` (`table.proof.rs:21`) and three `uninterp spec fn`
(`table.spec.rs:61,63,83`). Both patterns are listed as **Banned** (unconditional)
by the `verus-constraints` skill, and the task's hard guardrail states *"ANY
admit>0 OR assume>0 is a BLOCKER."* In addition, the bug/TCB documentation
describes `lemma_entry_roundtrip` as `external_body (empty body)` while the code
actually uses `assume(...)` — a doc/code inconsistency. Details below.

---

## Checklist (consolidated)

### Caller Analysis
- [x] Real callers identified beyond LSP false-negative — `caller_analysis.md` recovers
  `identity_map.rs` and `gva.rs` callers by source search; justified the LSP "0 callers"
  artifact (x86 vs x86_64 indexing). Solid.

### View Design
- [x] View structs justified with substitution test — `view_design.md` gives `TableIndex@:nat`
  and `TableView<E>{addr,entries}` with rejected alternatives. Good.
- [ ] View realized soundly without banned primitives — the final `TableView::entries` map is
  built from `spec_table_read` → `spec_table_word`, an **`uninterp spec fn`** (`table.spec.rs:83`),
  and the round-trip relies on an **`assume`** axiom. The View "works" only by leaning on
  patterns the skill bans (see Guardrails).

### Specification
- [x] Every in-scope exec fn has contracts — `fn_coverage.py`: 7/7 matched, 0 missing.
- [ ] Spec quality free of banned constructs — `spec_entry_raw`/`spec_entry_from_raw`/
  `spec_table_word` are `uninterp spec fn` (banned: "all spec functions must have concrete
  definitions"). The skill explicitly calls out `uninterp` + `external_body`/axiom pairing as
  "the same effect as assume" — which is exactly this design.
- [ ] `write` caller round-trip covered — `write` carries **no** contents `ensures`; the central
  caller invariant "read after write yields `Some(entry)`" is *deferred*, not proven (see
  Caller Coverage).

### Proving
- [x] `make verify-arch` exit 0, 0 errors — latest real verification log
  (`verus_2026-06-15_13-53-59.log`) = `48 verified, 0 errors`; my run re-confirmed via cache
  (exit 0). AST consistent, no spec drift.
- [ ] Proof obligations discharged without `assume` — `lemma_entry_roundtrip` is discharged by
  `assume(...)` at `table.proof.rs:21`, not by a real proof or an approved TCB mechanism.

### Cheating Elimination
- [x] Automated gate passes — `cheating: assume=0 external_body=3 admit=0 trusted=0
  no_decreases=0 cfg_gate=4`.
- [ ] Strict skill compliance — the gate's `assume=0` is achieved via the framework's
  `limitation_assume (approved id=L1)` carve-out; the **raw** `assume(` count is 1. Under the
  `verus-constraints` skill (which bans `assume` *unconditionally*, with no carve-out) this is a
  violation. `uninterp spec fn` ×3 are likewise banned by the skill and not gate-counted.

### Bug Recording
- [ ] `bugs.md` / `tcb-allowed.md` consistent with code — **NO.** Both documents state
  `lemma_entry_roundtrip` uses `external_body (empty body)` and `bugs.md` asserts `assume=0`,
  but the code uses `assume(...)`. Documentation describes a mechanism not present in the source.

---

## Spec Quality

The external-facing contracts that *are* present are clean, declarative, and caller-oriented:

- `pd_index`/`pt_index` (`table.rs:101-134`): `ensures result@ == spec_pd_index/pt_index(vaddr)`
  and `result@ < PAGE_TABLE_LENGTH`. Spec functions (`spec_table_index`, `table.spec.rs:36-48`)
  mirror the masking declaratively. Good.
- `TableIndex::into_raw` (`table.rs:85-93`): `ensures result as nat == self@`,
  `result < PAGE_TABLE_LENGTH`. Identity projection plus the validated bound. Good.
- `TableIndex` type invariant (`table.spec.rs:24-28`): `inv = self@ < PAGE_TABLE_LENGTH` — the
  exact validated-range guarantee callers depend on.
- `TableIndex::new` (`table.rs:65-78`): Some/None contract on the `< LEN` bound. Good (not strictly
  in the named scope, but specified).
- `from_address` (`table.rs:174-182`): `ensures result@.addr == base as nat`. Faithful, minimal.
- `read` (`table.rs:203-208`): `requires index@ < LEN`, `ensures result ==
  spec_table_read::<E>(self@.addr, index@)`. Reasonable *as an assumed contract* (it is
  `external_body`).
- `TableEntry::raw`/`from_raw` (`table.rs:33-41`): pinned to `spec_entry_raw`/`spec_entry_from_raw`.

Quality problems:

1. **Banned `uninterp spec fn` underpin the whole content model** (`table.spec.rs:61,63,83`).
   The skill: *"`uninterp spec fn` — Banned — all spec functions must have concrete definitions.
   Using `uninterp` to avoid writing a concrete spec is cheating — it has the same effect as
   `assume` when paired with `external_body` proof axioms."* Here all three are uninterp **and**
   paired with the `assume`-based `lemma_entry_roundtrip` — precisely the banned combination.
2. **`write` has no contents postcondition** (`table.rs:241-245`). The documented reason
   (Turn 2 correction) is genuinely sound — pinning a *pure* `spec_table_word` to `entry` under an
   assumed `external_body` contract is unsound. But the consequence is that the **key caller
   invariant "read-after-write round-trip" is not provable from this module's contracts**; it is
   deferred to a future permission token that does not exist yet.

---

## Caller Coverage

`caller_analysis.md` enumerates these caller expectations / key invariants:

| # | Caller expectation | Mapped to contract? | Where |
|---|--------------------|---------------------|-------|
| 1 | `pd_index` = masked bitfield, in range, total | ✅ | `table.rs:101-105` |
| 2 | `pt_index` = masked bitfield, in range, total | ✅ | `table.rs:119-123` |
| 3 | `TableIndex` validated `< LEN`; `into_raw` identity | ✅ | `table.spec.rs:25`, `table.rs:85-89` |
| 4 | `from_address` → handle over `base`, phantom `E` | ✅ | `table.rs:174-176` |
| 5 | `read` index→entry map, `None`=invalid | ✅ (assumed via `external_body`) | `table.rs:203-208` |
| 6 | `write` read-after-write round-trip; only slot `i` changes | ❌ deferred (no `ensures`) | `table.rs:241-245` |
| 7 | `TableEntry` round-trip law | ⚠ lemma exists but unusable for write path | `table.proof.rs:16-22` |

**Covered: 5/7 fully; 1 partial (#7); 1 missing (#6).**

- Missing #6: the "Read/write round-trip" key invariant (`caller_analysis.md:170-171`) is **not**
  established. `write` does not pin `spec_table_word(addr,i)` to `entry`, so no caller can derive
  `read(i) == Some(entry)` after `write(i,e)`. This is acknowledged/deferred in `view_design.md`
  (Turn 2 Correction) and `tcb-allowed.md`, but it remains an unfulfilled caller expectation.
- Partial #7: `lemma_entry_roundtrip` proves `dec(enc(e)) == Some(e)`, but because #6 is missing
  there is no path linking a `write` to a later `read`, so the round-trip lemma cannot actually
  deliver the caller's read-after-write guarantee. It is also itself discharged by `assume`.

---

## Proof Completeness

- `admit()` count: **0** (verified: `grep` of all three files; gate `admit=0`).
- `external_body` on own-module functions: **2** — `table.rs:202` (`Table::read`),
  `table.rs:241` (`Table::write`). Both are present in `tcb-allowed.md` (lines 37-58).
  - Note: `lemma_entry_roundtrip` is **NOT** `external_body` in the code (contrary to
    `tcb-allowed.md:64-65` and `bugs.md:37`); it is an `assume` (`table.proof.rs:21`).
- `external_body` not in TCB: **0** (both own-module `external_body` are TCB-listed).

Proof-completeness blocker: the codec injectivity obligation is closed with
`assume(spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e))` (`table.proof.rs:21`) rather
than a proof or an approved external axiom. The accompanying L1 reproducer (`repros/L1.rs`) does
demonstrate that two `uninterp` functions over a structureless generic `E` cannot be related
in-module — i.e. the *limitation is real*. However, the skill's prescribed remedy is **not**
`assume`: `assume` is unconditionally Banned. The honest, skill-compliant route would be a
concrete (interpreted) codec model or an `external_body` broadcast axiom **explicitly TCB-listed
as such** — and the docs even *claim* that is what was done, but the code does not match.

---

## TCB Compliance

- `Table::read` (`table.rs:202`): in `tcb-allowed.md:37-46`. ✅ Listed. Rationale (int-to-ptr
  volatile load) is legitimate and matches `frame::instance`/`bump_allocator::alloc` precedent.
- `Table::write` (`table.rs:241`): in `tcb-allowed.md:47-58`. ✅ Listed.
- `lemma_entry_roundtrip` (`table.proof.rs:16`): `tcb-allowed.md:59-66` lists it **as
  `external_body` (empty body)** — but the code uses `assume(...)`. ❌ **Inconsistent**: the TCB
  entry does not describe the mechanism actually in the source. An `assume` inside a proof body is
  *not* an `external_body` and is not a recognized TCB primitive (the skill's TCB section permits
  `assume_specification`, `external_body`, `axiom` on allowed-list items — **not** `assume`).

**TCB verdict: NO (partial).** The two int-to-ptr `external_body` are properly listed; the
`lemma_entry_roundtrip` listing misdescribes the actual code (assume vs external_body).

---

## Guardrails Compliance (exact counts, scope = 3 module files)

```
admit:               0
assume:              1   (table.proof.rs:21)            [gate reports 0 via id=L1 carve-out]
external_body:       2   (table.rs:202 read, table.rs:241 write)   [both TCB-listed]
uninterp spec fn:    3   (table.spec.rs:61, 63, 83)
assume_specification:0
cfg-gated exec:      0   (table.rs:9,11 are #[cfg(verus_keep_ghost)] on include! of
                          spec/proof files — non-semantic, sanctioned pattern, not exec gating)
```

Cross-check vs automated gate (`cheating-detail.txt`, whole crate):
```
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=4
  - x86/mem/paging/mod.rs:80      invlpg:               external_body   (TCB-listed)
  - x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip: assume      (detail-listed, NOT counted)
  - x86/mem/paging/table.rs:209   read:                 external_body   (TCB-listed)
  - x86/mem/paging/table.rs:246   write:                external_body   (TCB-listed)
```

The `assume` at `table.proof.rs:16/21` **is** detected by the gate (it appears in
`cheating-detail.txt`) but is **excluded from the `assume=0` total** by the framework's
`limitation_assume (approved id)` carve-out. `cfg_gate=4` are the spec/proof `include!` guards
(non-semantic), not exec cfg-gating.

**Two interpretations (as required by the task):**
- *Framework/guardrails.py view:* `assume=0` (id=L1 carve-out) and all `external_body` TCB-listed
  ⇒ the automated gate **passes**.
- *`verus-constraints` skill view:* `assume(...)` is **Banned unconditionally** (Forbidden
  Patterns table, no carve-out) and `uninterp spec fn` is **Banned unconditionally** ⇒
  **violation**.

**Strict verdict (this review follows the skills + the task's hard guardrail):** the presence of
`assume>0` in the raw source is a **BLOCKER**. The 3× `uninterp spec fn` are an additional
skill-banned pattern (and the exact "uninterp + axiom = assume" anti-pattern the skill names).

---

## AST Consistency

**PASS.** `ast_consistency.py --base-ref verus-ai-prove ... count` → `✅ Consistent: 7 functions,
2 structs match.` No `// VERUS REWRITE` / `VERUS DEVIATION` / `VERUS BUG FIX` comments present
(grep: none). `spec_drift.py` → 0 contract drift, 0 ensures removed, 0 requires added. Exec code
is byte-faithful to the base. Good.

---

## Verification

**PASS (0 errors).** `make verify-arch` exit code **0**. Latest substantive verification log
(`verus_2026-06-15_13-53-59.log`): `verification results:: 48 verified, 0 errors`. My run was
served from cache (source unchanged) and re-ran the cheating gate on current source. Coverage:
18/525 crate exec fns have contracts (7/7 for this module). No warnings reported in the gate.

(Historical note: `bugs.md` claims "47 verified"; the current source verifies 48 — minor stale
count, not a defect.)

---

## Bug Summary

- `bugs.md` records: **0 code bugs** (explicitly "No code bugs were found"). I concur — no true
  code bug exists in the in-scope functions; the obstacles are a real Verus int-to-ptr limitation
  (`read`/`write`) and the genuine uninterp-generic codec limitation (L1).
- True bugs found by this review: **0 code bugs**.
- **Documentation/process defects found (not in bugs.md):**
  1. `bugs.md:37` and `tcb-allowed.md:64-65` claim `lemma_entry_roundtrip` is `external_body
     (empty body)`; the code uses `assume(...)` (`table.proof.rs:21`). `bugs.md:38` claims
     `assume=0`; raw source has `assume=1`. **Inconsistency was not recorded as a finding.**
  2. The deferred `write` round-trip (caller expectation #6) is documented as a deferral but its
     impact — that the module's headline "read/write round-trip" caller invariant is currently
     *unproven* — is presented as benign ("lose no concrete verification value today because all
     callers `admit()`"). That is a coverage gap worth flagging explicitly.

---

## Issues (highest priority first)

1. **[BLOCKER] `assume(...)` in proof body** — `table.proof.rs:21`.
   Banned unconditionally by `verus-constraints`; task hard-guardrail: any `assume>0` is a
   BLOCKER. The gate hides it behind the `limitation_assume id=L1` carve-out, but the raw count
   is 1. The L1 limitation is *real*, but the skill-sanctioned remedy is a concrete codec model or
   a properly-declared `external_body` broadcast axiom — not `assume`.

2. **[BLOCKER] `uninterp spec fn` ×3** — `table.spec.rs:61,63,83`
   (`spec_entry_raw`, `spec_entry_from_raw`, `spec_table_word`).
   Banned by `verus-constraints` ("all spec functions must have concrete definitions"). Combined
   with the assume axiom this is exactly the "uninterp + axiom ≈ assume" cheating pattern the
   skill names. The entire `TableView::entries` content model rests on these.

3. **[HIGH] Doc/code inconsistency for `lemma_entry_roundtrip`** — `tcb-allowed.md:64-65`,
   `bugs.md:37-39` say `external_body (empty body)` and `assume=0`; code uses `assume`. The TCB
   list therefore "approves" a mechanism that is not the one in the source. `assume` is not a TCB
   primitive (the skill allows `assume_specification`/`external_body`/`axiom` only).

4. **[MEDIUM] Missing caller invariant: read-after-write round-trip** — `write` (`table.rs:241`)
   has no contents `ensures`; caller expectation #6 / key invariant "Read/write round-trip"
   (`caller_analysis.md:170`) is unprovable from this module. The soundness reasoning for omitting
   it is correct, but the round-trip remains genuinely deferred (no permission token exists), so
   the module does not yet deliver its central content guarantee.

5. **[LOW] Stale verified count in `bugs.md`** — claims 47; current source verifies 48. Cosmetic.

---

## Result: **FAIL**

PASS requires *all* checklist items to pass. The following checklist items are unchecked:
Specification (banned `uninterp`; missing `write` round-trip), Proving (obligation closed by
`assume`), Cheating Elimination (raw `assume>0`; skill-banned `uninterp`), Bug Recording
(TCB/bugs docs inconsistent with code). The automated `make verify-arch` gate passes and AST/spec
fidelity are clean, but under the `verus-constraints` skill and the task's explicit hard guardrail
("ANY admit>0 OR assume>0 is a BLOCKER"), the `assume(...)` at `table.proof.rs:21` plus the three
banned `uninterp spec fn` make this a **FAIL**.

### Minimal path to PASS
- Replace the `assume` in `lemma_entry_roundtrip` with a genuinely skill-compliant mechanism:
  either give `spec_entry_raw`/`spec_entry_from_raw` concrete (interpreted) definitions so the
  round-trip is *proved*, or declare the lemma as an explicit `external_body` broadcast axiom and
  make `tcb-allowed.md`/`bugs.md` accurately say so (eliminating the doc/code mismatch). The latter
  still leaves the `uninterp spec fn` skill-ban to resolve.
- Provide concrete definitions for the three `uninterp spec fn` (or move them behind a properly
  TCB-listed external-bottom boundary with documented justification matching the code).
- Reconcile `bugs.md` / `tcb-allowed.md` with the actual source (assume vs external_body;
  assume count; verified count).
