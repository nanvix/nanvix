## Response to Turn 2

### Confirming the hole first

Before changing anything I reproduced the reviewer's unsoundness claim in a
scratch Verus crate (`external_body` `write` with the Turn-1 contents `ensures`,
plus `lemma_entry_roundtrip`):

```rust
proof fn exploit<E>(e1: E, e2: E) requires e1 != e2 {
    assume(spec_table_word(a, i) == spec_entry_raw(e1));   // write(i,e1)
    assume(spec_table_word(a, i) == spec_entry_raw(e2));   // write(i,e2)
    assert(spec_entry_raw(e1) == spec_entry_raw(e2));
    lemma_entry_roundtrip(e1); lemma_entry_roundtrip(e2);
    assert(Some(e1) == Some(e2));
    assert(e1 == e2);
    assert(false);                                          // VERIFIES
}
```
→ `verification results:: 3 verified, 0 errors`. The `assert(false)` **verified**,
confirming the soundness hole exactly as described (#2/#3/#15). The Turn-1
scratch check only exercised a single write, which hid it.

### What I Changed

`src/libs/arch/src/x86/mem/paging/table.rs` — `Table::<E>::write` (≈ lines 224–238):

- **Removed** the unsound contents postcondition
  `ensures spec_table_word(self@.addr, index@) == spec_entry_raw(entry)`.
- **Kept** only the sound `requires index@ < crate::mem::PAGE_TABLE_LENGTH`
  (auto from `TableIndex::inv`).
- **Rewrote the trust-boundary comment** to explain *why* there is no contents
  `ensures`: `spec_table_word` is a *pure* `uninterp spec fn`, so pinning it to
  the caller-chosen `entry` in an assumed (`external_body`) postcondition lets two
  writes of distinct entries to the same slot derive `false` via
  `lemma_entry_roundtrip`. The genuine `old@ -> @` slot-update transition
  (`self@.entries[index@] == Some(entry)`, other slots framed) is **deferred to
  the proving-phase page-table permission token**, citing the
  `identity_map_view()` `v -> v'` deferral convention in `identity_map.spec.rs`.

New `write` contract:
```rust
#[verus_verify(external_body)]
#[verus_spec(
    requires
        index@ < crate::mem::PAGE_TABLE_LENGTH,
)]
pub unsafe fn write(&self, index: TableIndex, entry: E) { ... }
```

Left unchanged (all sound, per the request): `read`'s `ensures result ==
spec_table_read::<E>(self@.addr, index@)`, the `raw`/`from_raw` ensures,
`lemma_entry_roundtrip`, and the `TableView<E> { addr, entries }` view.
(`spec_table_word`/`spec_entry_raw` remain referenced by `read`/`raw`, so no dead
spec code.)

Supporting docs updated to match: `tcb-allowed.md` (write entry now says
"no contents ensures, deferred"), `view_design.md` (new "Correction (Turn 2
review)" section), `verus-unsupported.md` (deferred-work paragraph).

### Verification

**`make verify-arch`** → exit 0:
```
=== Summary ===
  verification: 23 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=2 admit=1 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 9/525 exec functions have contracts
```

**`make verify`** (full regression — no cascade) → exit 0:
```
arch:   verification results:: 23 verified, 0 errors
kernel: verification results:: 76 verified, 0 errors
```

**`./z build`** (dual compilation) → `[OK] Build complete.` (exit 0).

**`spec_drift.py check nanvix-phys-arch-paging-table`** → 0 contract drift
(baseline already at the committed pre-fix HEAD; the removal of an
*unsound* ensures is the intended correction per checklist #15 — replacing an
unsound-stronger contract with a sound one).

**`fn_coverage.py table.rs table.rs`** → 7 matched, 0 missing (all exec fns still
carry contracts: `from_address, into_raw, new, pd_index, pt_index, read, write`).

After the fix, the same exploit can no longer be written: with no `ensures` on
`write`, a caller learns *nothing* about `spec_table_word` from a write, so the
two `assume`s above are no longer available and `false` is no longer derivable.

### Result: FIXED
