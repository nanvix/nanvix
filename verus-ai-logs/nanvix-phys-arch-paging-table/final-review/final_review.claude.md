# Final Comprehensive Review: arch-paging-table (claude-opus-4.8)

Module: `arch::x86::mem::paging::table`
Reviewer: independent strict final verification review
In-scope functions: `Table::write`, `TableIndex::into_raw`, `TableEntry::raw`,
`Table::read`, `TableEntry::from_raw`, `pt_index`, `TableIndex` (type + `new`),
`pd_index`, `Table::from_address`.

Independently re-run read-only checks (all reproduced the orchestrator's results):

- `ast_consistency.py --base-ref 07eb0d8e4 … summary` → `Consistent: ✅ YES
  (matched=7 mismatched=0 missing=0 extra=0)`; structs `Table`, `TableIndex` MATCH.
- `spec_drift.py git-diff … --before HEAD` → `✅ No contract drift detected`
  (ensures removed 0, requires added 0).
- grep guardrails over `table.rs` / `table.spec.rs` / `table.proof.rs`:
  `admit` 0, `assume(` 0, `assume_specification` 0, `external_body` = 2 attribute
  sites (lines 202 `read`, 241 `write`; a third hit at 231 is prose in a comment),
  `exec_allows_no_decreases` 0, `spinoff_prover` 0, `rlimit` 0,
  `cfg(not(verus_keep_ghost))` 0, `VERUS REWRITE` 0.
- `verus-logs/cheating-detail.txt` lists exactly the 3 arch-crate `external_body`:
  `invlpg`, `table::read`, `table::write` — all three TCB-listed.
- Latest `verify-arch` log: `Exit code: 0`; cheating `assume=0 external_body=3
  admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
- Constants confirmed from `x86/mem/constants.rs`: `PAGE_SHIFT=12`,
  `PGTAB_SHIFT=22`, `PAGE_TABLE_LENGTH = PGTAB_SIZE/PAGE_SIZE = 4MiB/4KiB = 1024`,
  mask `LEN-1 = 1023`. `PteWord = u32`. The spec arithmetic matches the exec masks
  exactly.

---

## Checklist

### Caller Analysis
- [x] All pub functions have callers searched. `caller_analysis.md` recovers the
  true callers textually (kernel `mm/virt/identity_map.rs`, host
  `uservm/.../gva.rs`) and correctly explains the LSP false-negative (this is the
  32-bit x86 tree; rust-analyzer indexes the x86_64 host config).
- [x] Caller expectations documented for success **and** failure paths (per-function
  "Caller Expectations" + "Key Invariants" sections; `None`→`InvalidArgument`
  mapping recorded).
- [x] Abstract resource identified: a single hardware page-table page as a partial
  map `index → Option<E>` over caller-owned volatile memory, plus the pure
  index extractors.
- [x] Pre-existing specs assessed: the inherited `identity_map.spec.rs` boundary
  `assume_specification`s (state-free, `admit()`-backed) are explicitly judged
  weak/partial and superseded by the real contracts written here.

### View Design
- [x] Every field passes the substitution test: `TableIndex@:nat`,
  `TableView{addr:nat, entries:Map<nat,Option<E>>}` are all algorithm-free; the
  rejected mirror-the-struct alternative (`base:usize`, `PhantomData`) is correctly
  excluded.
- [x] All caller-observable state represented: `addr` (which page) + `entries`
  (per-slot decoded entry, the read result).
- [x] No impl-specific fields: `base`/`PhantomData` deliberately excluded from the
  View.
- [x] `inv()` encodes a real constraint: `TableIndex::inv` is the
  `type_invariant self@ < PAGE_TABLE_LENGTH` — the validated-index guarantee every
  caller depends on.
- [x] Mathematical types used: `nat`, `Map<nat,Option<E>>`.
  - Advisory: the view_design "Well-formedness Invariants" section proposed a
    `TableView::inv` (domain `=~= [0,LEN)` + `addr % PAGE_SIZE == 0`). As built,
    the domain is baked directly into `view()` (`Map::new(|i| i < LEN, …)`) and
    page-alignment is **not** enforced anywhere (`from_address` accepts any
    `base`). This is acceptable — alignment is a caller-carried `unsafe` safety
    obligation, not a verifiable property of the non-owning handle — but it is a
    documented divergence from the design note worth flagging.

### Specification
- [x] Every in-scope exec fn carries `#[verus_spec]` contracts:
  `new` (Some/None), `into_raw` (identity+bound), `pd_index`/`pt_index`
  (value+bound), `from_address` (addr), `read` (decode), `write` (requires only),
  `raw`/`from_raw` (codec pin).
- [~] Caller coverage: **11 / 13** caller expectations covered. The 2 uncovered are
  the **read-after-write round-trip** and the **`from_raw(raw(e)) == Some(e)`
  codec law** — both consciously **deferred** (see Caller Coverage + Issues). I
  judge this an *acceptable, soundly-motivated, fully-documented* deferral, **not**
  a blocker (justified below), so this item is treated as satisfied at the maximal
  *sound* coverage achievable in this phase.
- [x] View consistency: `read`/`from_address` ensures reference `self@.addr`; the
  closed `view()` still exposes `addr` to callers through `from_address`'s ensures,
  so the `from_address → read` chain composes.
- [x] No tautological ensures (every clause constrains the result/state).
- [x] No subsumed ensures: e.g. `pd_index`/`pt_index` carry both
  `result@ == spec_…` *and* `result@ < LEN`; the bound is not implied by the value
  equation alone (it needs mask reasoning), so it is a meaningful extra fact for
  callers (`gva.rs`'s `checked_mul`).
- [x] Error paths meaningful: `new` has an explicit `None ⇒ index >= LEN` arm;
  `read` returns `None` exactly when `spec_entry_from_raw` rejects the word.
- [x] No `assume_specification` for workspace-internal code (0 in the module).
- [x] vstd searched / appropriate math types used.
- [x] Specs written for the caller: `result@`, `self@.addr`, `spec_table_read`,
  `spec_pd_index`/`spec_pt_index` are directly usable in caller proofs.
- [x] Trait obligations satisfied: `TableEntry` methods `raw`/`from_raw` pinned to
  `spec_entry_raw`/`spec_entry_from_raw`; codec abstraction is `E`-unbounded to
  avoid a trait↔function definitional cycle (a sound, well-reasoned choice).
- [~] Spec completeness advisory: the write-transition + codec round-trip law are
  recorded as proving-phase deferrals (advisory acknowledged, not silently
  dropped).
- [x] Loop invariants: N/A (no loops in scope).
- [x] No cheating counts: admit 0 / assume 0 / trusted 0 / no_decreases 0 /
  cfg_gate 0.
- [x] No specs weakened: `spec_drift` exit 0 vs HEAD.
- [x] Bug awareness: `bugs.md` present and reasoned.
- [x] Cross-module regression: `make verify` (all crates) exit 0 per orchestrator.
- [x] Verification: `make verify-arch` exit 0; `./z build` exit 0.

### Proving
- [x] No specs weakened (spec_drift exit 0).
- [x] Zero `admit()`.
- [x] Zero `external_body` except TCB-listed (`read`, `write` both listed).
- [x] Zero `assume` / `assume_specification`.
- [x] No cfg-gated exec (the two `#[cfg(verus_keep_ghost)]` at lines 9/11 guard the
  `include!` of the ghost-only `.spec.rs`/`.proof.rs`, not exec code).
- [x] Cheating audit counts + locations recorded (below).
- [x] Claimed Verus limitation has an isolated reproducer: `verus-unsupported.md`
  gives the minimal `usize → *const u32` cast and the exact Verus error.
- [x] Exec rewrites minimal + equivalent: there are **none** (AST MATCH on all 7
  fns + 2 structs); the `proof!` blocks in `pd_index`/`pt_index`/`into_raw` are
  ghost-only.
- [x] Cross-module regression green.
- [x] Verification 0 errors / 0 warnings (verify-arch exit 0, no warnings in log).

### Cheating Elimination
- [x] Zero `admit`.
- [x] Zero `assume`.
- [x] Zero `trusted`.
- [x] Zero `exec_allows_no_decreases_clause`.
- [x] Zero cfg-gated exec.
- [x] Zero `external_body` except TCB-listed.
- [x] AST consistency zero mismatches.
- [x] All exec rewrites have a VERUS REWRITE comment + reproducer: vacuously true —
  there are zero exec rewrites (grep `VERUS REWRITE` = 0; AST MATCH).
- [x] Each surviving `external_body` is TCB-listed (`read`, `write`).
- [x] No specs weakened.
- [x] Cross-module regression green.
- [x] Verification green.

### Bug Recording
- [x] `bugs.md` exists; states "No code bugs" with reasoning.
- [x] Each recorded item is a genuine *non-bug* (Verus int-to-ptr limitation +
  deferred abstraction), correctly classified as **not** a code defect per the
  bug-reporting skill (False-Positive / language-limitation category, not True
  Bug / Context-Dependent).
- [x] Provenance / "How Verus Helped" captured: the Turn-2 unsoundness finding
  (pinning the pure ghost in `write` derives `false`) is a real verification-only
  insight, recorded across `bugs.md` / `verus-unsupported.md` / `view_design.md` /
  `tcb-allowed.md`.
- [x] No `external_body` masking a defect: both boundaries are genuine
  external-bottom hardware/Verus-limitation trust boundaries, not hidden proof
  failures.

Legend: `[x]` satisfied, `[~]` satisfied at the maximal *sound* level with a
documented deferral (see Issues). No `[ ]` (unchecked) items remain — see the
PASS justification in Result.

---

## Spec Quality

Strong and idiomatic. Highlights:

- **`TableIndex` / `new` / `into_raw`** — textbook. The validated-index guarantee
  is a `type_invariant` (`self@ < LEN`), established at every construction site
  (`new`, `pd_index`, `pt_index`) and consumed by `into_raw` via
  `use_type_invariant`. `new` has both Some/None arms; `into_raw` is the identity
  projection plus the bound. Substitution-clean, caller-friendly.
- **`pd_index` / `pt_index`** — `spec_pd_index`/`spec_pt_index` mirror the exec mask
  `(vaddr >> shift) & (LEN-1)` exactly with the correct shifts (22 / 12), and the
  `result@ < LEN` bound is discharged by a `by (bit_vector)` proof. Independent of
  the implementation and total. Excellent.
- **`from_address`** — `result@.addr == base`, the only observable; `unsafe`
  correctly carries the page-validity obligation to the caller.
- **`read`** — full decode contract `result == spec_table_read::<E>(self@.addr,
  index@)` pinned to a global, parameter-free page-table-memory ghost
  (`spec_table_word` → `spec_entry_from_raw`), faithfully mirroring the
  `frame::instance → phys_view()` precedent. Reading a pure accessor is sound
  (two reads agree). This is the right design.
- **`raw` / `from_raw`** — pinned to the uninterpreted codec; the `E`-unbounded
  abstraction to dodge the trait↔function cycle is correct and well-explained.

**Use of `uninterp spec fn`** (`spec_entry_raw`, `spec_entry_from_raw`,
`spec_table_word`): the `verus-constraints` skill lists `uninterp` as "banned when
used to avoid writing a concrete spec." That prohibition does **not** bite here:
these model genuinely *external-bottom* state — caller-owned volatile page-table
memory and a per-implementor bit-level codec that lives outside this module — which
is exactly the established project convention (`phys_view`, `identity_map_view`,
`byte_at_address`, `bump_view`, `raw-array::view`, `upool::view` are all `uninterp`).
This is modeling-of-external-state, not spec-avoidance. Acceptable.

**`write`** — carries only `requires index@ < LEN` and **no** contents `ensures`.
This is the single substantive limitation and is analyzed next.

---

## Caller Coverage

- Covered: **11 / 13**

| # | Caller expectation (caller_analysis.md) | Spec evidence | Covered |
|---|------------------------------------------|---------------|---------|
| 1 | `pd_index` = `(vaddr>>22)&1023`, total, in-range | `ensures result@==spec_pd_index(vaddr) && result@<LEN` | ✅ |
| 2 | `pt_index` = `(vaddr>>12)&1023`, total, in-range | `ensures result@==spec_pt_index(vaddr) && result@<LEN` | ✅ |
| 3 | `TableIndex` validated `<LEN` | `type_invariant inv: self@<LEN` | ✅ |
| 4 | `into_raw` loss-less identity projection | `ensures result as nat==self@ && result<LEN` | ✅ |
| 5 | `new` Some/None on the `<LEN` bound | `ensures` both arms | ✅ |
| 6 | `from_address` handle denotes page `base` | `ensures result@.addr==base` | ✅ |
| 7 | `read` decodes slot, `None`=invalid encoding | `ensures result==spec_table_read::<E>(addr,index)` | ✅ |
| 8 | Decode totality (zeroed entry is `Some`, not `None`) | encoded in `spec_entry_from_raw` semantics (decode definition) | ✅ |
| 9 | Type discipline (phantom `E` separates PD/PT) | `Table<E>` / `TableView<E>` parameterization + type system | ✅ |
| 10 | Non-ownership (`from_address` allocates nothing) | `unsafe` + addr-only contract | ✅ |
| 11 | Index-extraction round-trip (`into_raw` of an extractor == masked value) | items 1/2/4 compose | ✅ |
| 12 | **Read-after-write round-trip** (`write(i,e)` then `read(i)`==`Some(e')`; only slot `i` changes) | **none** — `write` has no contents `ensures` | ❌ deferred |
| 13 | **Codec round-trip law** (`from_raw(raw(e))==Some(e)`) | **none** — `lemma_entry_roundtrip` was intentionally removed | ❌ deferred |

- Missing: **[12 read-after-write write-transition, 13 codec round-trip law]** —
  both deferred to the proving phase.

**Assessment of the `write` / round-trip deferral (the key question).**
This is an **acceptable documented deferral, NOT a blocker**, for these reasons:

1. **The only alternative is unsound.** `spec_table_word` is a *pure*
   `uninterp spec fn` (one fixed value per slot). Because `write` is
   `external_body`, any `ensures` is *assumed* at every call site. Pinning the
   pure cell to the caller-chosen `entry` lets two distinct writes to the same
   slot assume `spec_entry_raw(e1)==spec_entry_raw(e2)`, deriving `false`. This
   was reproduced (`assert(false)` verified) in Turn 2 and is a *correct*
   verification-only finding. Adding a wrong/unsound ensures would be strictly
   worse than omitting it.
2. **The sound mechanism is genuinely out-of-scope.** Expressing the mutable
   `old@ → @` slot-update soundly requires a `PointsTo`-style page-table
   permission token threaded through `read`/`write`, which cascades a ghost
   parameter into out-of-scope callers (`identity_map::ensure_pt`/`ensure_pte`/
   `identity_map_page`) that currently begin with `admit()`. That is exactly the
   established project deferral convention (`identity_map_view()`'s `v → v'`,
   `phys_view()`'s "transition realized in the proving phase").
3. **No verified caller depends on it.** Every real caller of the round-trip is
   itself `admit()`-stubbed today, so nothing downstream is silently broken.
4. **It is honestly recorded** in `bugs.md`, `verus-unsupported.md`,
   `view_design.md` (Turn-2 correction), `tcb-allowed.md`, and the inline `write`
   comment — never masked.

The maximal *sound* contract for `write` in this phase is precisely
`requires index@ < LEN` with no contents `ensures`. The module delivers that.

---

## Proof Completeness
- Remaining `admit()`: **0** (grep clean; verify-arch `admit=0`).
- Remaining `external_body` not in `tcb-allowed.md`: **0**. The only two in this
  module — `Table::read` (table.rs:202) and `Table::write` (table.rs:241) — are
  both explicitly listed in `tcb-allowed.md` (the "introduced while speccing
  arch::…::table" section). `table.proof.rs` contains **no** lemmas/axioms (it is
  entirely explanatory comments — the removed `lemma_entry_roundtrip` placeholder
  is correctly absent rather than left as an unproven axiom).

## TCB Compliance
- All `external_body` listed in `tcb-allowed.md`: **YES**. `read` and `write` are
  both listed, each justified by the genuine `usize → *const/*mut T` Verus
  limitation (documented with a minimal reproducer + exact error). No new trust
  boundary was introduced. (`invlpg` in the sibling `mod.rs` is the only other
  arch `external_body` and is likewise listed; out of this file's scope.)

## Guardrails Compliance
- admit: **0**, assume: **0**, external_body: **2** (`read`, `write`; both
  TCB-listed), assume_specification: **0**, cfg-gated exec: **0**.
- Additionally: trusted **0**, exec_allows_no_decreases **0**, spinoff_prover
  **0**, rlimit **0**.

## AST Consistency
- AST check: **PASS** — `matched=7 mismatched=0 missing=0 extra=0`; structs
  `Table`/`TableIndex` MATCH; 0 `// VERUS REWRITE`. Exec code is byte-for-byte
  semantically identical to baseline `07eb0d8e4`; all added material is ghost
  (`#[verus_spec]`, `proof!` blocks, `verus!` spec/proof includes).

## Verification
- verus: **PASS** — `make verify-arch` exit 0, arch crate verified, 0 errors, 0
  warnings; cheating `assume=0 external_body=3 admit=0 trusted=0 no_decreases=0
  cfg_gate=0`. `make verify` (all crates) exit 0; `./z build` exit 0.
- spec_drift vs HEAD: 0 contract drift. (Pre-existing out-of-scope kernel-crate
  `admit`/`external_body`/`cfg_gate` are unrelated to this module.)

## Bug Summary
- Total bugs recorded: **0 code bugs** (`bugs.md` = "None", with two recorded
  *non-bug* notes: the Verus int-to-ptr limitation and the deferred
  write-transition abstraction).
- True Bugs: **0**.

I independently agree with the "no code bugs" claim: the in-scope exec bodies are
correct (index masks, shift offsets, decode-on-read, encode-on-write all match the
hardware contract), and the two recorded notes are a genuine tooling limitation
and a sound architectural deferral, not defects.

---

## Issues (highest priority first)

1. **(Advisory, non-blocking) `write` round-trip + codec law deferred.** Caller
   expectations #12/#13 are not provable by any verified caller in this phase.
   Justified: the sound encoding needs a proving-phase page-table permission token
   (out of scope), and the unsound shortcut derives `false`. Tracked for the
   proving phase. **Not a blocker** per the analysis above.
2. **(Minor doc staleness) Stale `lemma_entry_roundtrip` references.** The lemma
   was intentionally removed (correctly — it was an unproven axiom), but the inline
   comment at `table.rs:233` and `tcb-allowed.md:44` still cite it as if it exists
   ("with `lemma_entry_roundtrip` that derives `e1==e2`"). The soundness *argument*
   remains valid hypothetically, but the citation now points at a non-existent
   lemma. Recommend rewording to "(a future codec round-trip law would derive …)".
   Cosmetic; no effect on verification.
3. **(Minor design divergence) `TableView::inv` not implemented.** `view_design.md`
   proposed a `TableView::inv` (domain + `addr % PAGE_SIZE == 0`); as built the
   domain is folded into `view()` and page-alignment is unenforced (caller-carried
   `unsafe` obligation). Acceptable for a non-owning handle, but the design note and
   the as-built spec differ — worth a one-line reconciliation in `view_design.md`.

None of the above is a verification-integrity violation. Items 2–3 are
documentation/cosmetic.

---

## Result: PASS

**Justification.** Every hard guardrail is clean (admit 0, assume 0,
assume_specification 0, trusted 0, exec_allows_no_decreases 0, cfg-gated exec 0;
both `external_body` are TCB-listed). AST consistency is an exact MATCH (no exec
mutation, no `// VERUS REWRITE`), spec drift is zero, and `verify-arch` / full
`make verify` / `./z build` all pass with zero errors and zero warnings. The
in-scope success and error contracts are correct, non-tautological, caller-usable,
and independent of the implementation.

The single substantive gap — `write` lacking a contents `ensures`, leaving the
read-after-write round-trip (and the codec round-trip law) uncovered — is the
**maximal sound** contract achievable in this phase: the only alternative is a
demonstrably *unsound* assumed postcondition (it derives `false`), and the sound
realization requires out-of-scope proving-phase machinery (a page-table permission
token) following the project's established `phys_view`/`identity_map_view` deferral
convention. It blocks no verified caller and is transparently documented in five
places. I therefore classify it as an **acceptable documented deferral, not a
missing-property blocker**, and mark the corresponding checklist items satisfied at
the maximal-sound level. The remaining issues are cosmetic documentation nits.

Accordingly, no checklist item is left unsatisfied, and the module **PASSES** final
verification review.
