## Turn 2: Verifying the Turn-1 fix — read/write/raw/from_raw contracts

### Progress
- **FIXED (verified this turn):** #1 (all 7 exec fns now have contracts),
  #6 read/write arms, #9 (read/raw/from_raw now caller-usable), #10 (trait
  obligation: round-trip law now stated), #11 (advisory).
- **PASS (re-verified):** #4 no tautology, #5 no subsumed, #7 no
  assume_specification in module, #8 N/A, #12 no loops, #13 cheating counts
  (see note — admit is an accepted spec-phase convention here), #14 spec_drift=0,
  #16 cross-module (`make verify-arch` exit 0), #17 build (`./z build` exit 0).
- **Current FAIL: #2 / #3 / #15 — `Table::write`'s `ensures` is UNSOUND.**
- Remaining: re-verify the single `write` fix, then STOP.

### Verification (commands run + evidence)

**`make verify-arch`** → exit 0:
```
cheating: assume=0 external_body=2 admit=1 trusted=0 no_decreases=0
coverage: 9/525 exec functions have contracts
status: CHEATING_DETECTED
cheating-detail:
  table.proof.rs:8  lemma_entry_roundtrip: admit
  table.rs:209 read:  external_body
  table.rs:238 write: external_body
```
**`./z build`** → `[OK] Build complete.` (exit 0).
**`fn_coverage.py`** → 7/7 matched, 0 missing.
**`spec_drift.py check`** → 0 contract drift (guarantees only added). ✅

**#13 admit — ACCEPTED by precedent (not a fail).** I challenged the new
`admit=1` and then checked repo convention. `admit()`-bodied proof lemmas are the
established specification-phase pattern here: `bump_allocator/src/lib.proof.rs`
(header: *"Bodies are `admit()`…"*), `mm/phys/manager.proof.rs`,
`hal/.../phys.proof.rs`, `identity_map.proof.rs` all ship admitted lemma bodies
with the proof deferred to the proving phase. `lemma_entry_roundtrip` sits in
`table.proof.rs` and matches this pattern exactly. The two `external_body`
(`read`/`write`) are the genuine `usize -> *const/*mut` Verus limitation, logged
in `tcb-allowed.md`/`verus-unsupported.md`. **#13 PASS.**

**#1/#6/#9/#10 — FIXED.** `from_raw`, `raw`, `read`, `write` now carry
`#[verus_spec]` ensures tied to a global page-table-memory ghost
(`spec_table_word`/`spec_table_read`), the `TableEntry` round-trip law
(`lemma_entry_roundtrip`) is stated, and no exec signature changed → no cascade
into the out-of-scope `admit()` callers (confirmed: `make verify-arch` and
`./z build` both green). Read's accessor contract
`result == spec_table_read::<E>(self@.addr, index@)` is sound and matches the
project's own `identity_map_view()` convention (`identity_map.spec.rs:28-36`:
a global uninterp accessor "read like `self@`" during the spec phase, transition
deferred to a proving-phase ghost token — *"not a verification escape"*).

### THE REMAINING FAIL — `Table::write` ensures is unsound (#2/#3/#15)

`table.rs:231-237`:
```rust
#[verus_spec(
    requires index@ < crate::mem::PAGE_TABLE_LENGTH,
    ensures  spec_table_word(self@.addr, index@) == spec_entry_raw(entry),
)]
pub unsafe fn write(&self, index: TableIndex, entry: E) { ... }   // external_body
```

`spec_table_word(addr, index)` is a **pure** `uninterp spec fn` — one fixed value
per `(addr, index)`. Because `write` is `external_body`, this `ensures` is
**assumed** at every call site. Pinning a *pure* function to a caller-chosen
`entry` makes the spec contradict itself across two writes to the same slot.
Concrete derivation a caller can perform (all facts are assumed/broadcast):

```
write(i, e1)  ⟹  spec_table_word(a, i) == spec_entry_raw(e1)      [A1]
write(i, e2)  ⟹  spec_table_word(a, i) == spec_entry_raw(e2)      [A2]
A1, A2        ⟹  spec_entry_raw(e1) == spec_entry_raw(e2)
lemma_entry_roundtrip:
    spec_entry_from_raw(spec_entry_raw(e1)) == Some(e1)
    spec_entry_from_raw(spec_entry_raw(e2)) == Some(e2)
        ⟹  Some(e1) == Some(e2)  ⟹  e1 == e2
```
So any caller that writes two **distinct** entries to the same slot derives
`e1 == e2`, i.e. **`false`** — and from `false`, anything. This is a genuine
soundness hole, not a weak spec. It is latent today only because every upstream
caller starts with `proof! { admit(); }`; it becomes exploitable the moment a
real `write(i,e1); write(i,e2)` proof is attempted. The fixer's "scratch-crate"
check only exercised a *single* write (read-after-one-write), which is exactly
the case that hides this bug.

Root reason: page-table memory is **mutable** volatile state, but `write` takes
`&self` and the model represents the cell as a **pure** function. A faithful
write-transition needs a state that changes between calls (`old@ -> @`), which
requires either `&mut self` (wrong for a non-owning handle, would cascade) or the
deferred ghost permission token (proving phase, cascades into the out-of-scope
`admit()` callers). This is precisely the *Open Mechanism Note* / *As-Built
Decision* in `view_design.md`: the write→`entries` transition must be deferred.
The fixer reintroduced the `entries` transition into `write`'s `ensures` via a
pure-function pin, which is the unsound shortcut that note warned against.

Note this does **not** condemn `read`: reading a pure accessor is sound (two
reads agree; matches `identity_map_view()`'s deferral). The defect is isolated to
`write` asserting an equality on the global accessor.

### Fix Request (single, isolated change)

**Remove `write`'s contents `ensures`; keep only the sound `requires`.** Defer the
write→`entries` transition to the proving-phase ghost token, exactly as
`identity_map.spec.rs` defers `identity_map_view()`'s `v -> v'`.

In `src/libs/arch/src/x86/mem/paging/table.rs`, change `write` to:
```rust
#[verus_verify(external_body)]
#[verus_spec(
    requires
        index@ < crate::mem::PAGE_TABLE_LENGTH,
)]
pub unsafe fn write(&self, index: TableIndex, entry: E) { ... }
```
and update the trust-boundary comment to state that the slot-update transition
(`self@.entries[index@] == Some(entry)` after the call) is **deferred to the
proving-phase page-table permission token** (cite the `identity_map_view()`
deferral convention), so the omission is clearly intentional, not an oversight.

Keep unchanged (all sound): `read`'s `ensures`
(`result == spec_table_read::<E>(self@.addr, index@)`), the `raw`/`from_raw`
ensures, `lemma_entry_roundtrip`, and the `TableView { addr, entries }` view.

**Do NOT** instead try to "patch" by removing the round-trip lemma or by adding
raw-injectivity — those either drop a wanted caller guarantee (#10) or make the
contradiction *more* reachable. The correct move is to not pin a pure function in
`write`'s postcondition at all.

If you believe `write` can keep a **sound** contents postcondition without a
state token, that is a justification — prove it: write a scratch Verus client
that calls `write(i, e1); write(i, e2)` with `e1 != e2` (e.g. distinct via a
hypothesis), `broadcast use lemma_entry_roundtrip`, and `assert(false)`. If that
fails to verify, your spec is sound and I'll accept it. If it verifies (proves
`false`), it confirms the hole and you must apply the fix above.

**Verify the fix with:**
- `make verify-arch` → exit 0, must stay all-modules-verified.
- `python3 .../spec_drift.py check nanvix-phys-arch-paging-table` → the removed
  `ensures` will register as a drop; that is **intended** here (replacing an
  *incorrect/unsound* ensures, per checklist #15 "if a spec is incorrect, replace
  it with an equally strong correct spec" — a sound-weaker-but-correct contract
  beats an unsound-stronger one). Note the justification in the fixer reply.
- `./z build` → exit 0.

### Checklist status after Turn 2
- FIXED/PASS: #1, #4, #5, #6, #7, #8, #9, #10, #11, #12, #13, #14, #16, #17.
- **FAIL (fix requested): #2/#3/#15 — `write` unsound ensures.**
- STOP file **not** created — one FAIL remains.
