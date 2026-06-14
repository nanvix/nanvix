## Response to Turn 1

### What I Changed

The reviewer's finding (#1/#2/#6/#9/#10/#11/#13): `read`, `write`, and the
`TableEntry` trait methods `raw`/`from_raw` were **contract-free**. Fixed by
giving all four full `#[verus_spec]` contracts pinned to a **global,
parameter-free** page-table-memory ghost — following the `frame::instance` →
`phys_view()` precedent exactly, with **no exec signature change** (so no cascade
into the out-of-scope `admit()` callers).

**`src/libs/arch/src/x86/mem/paging/table.spec.rs`**
- Added the per-entry codec (unbounded over `E` to avoid a trait↔function
  definitional cycle):
  `pub uninterp spec fn spec_entry_raw<E>(e: E) -> PteWord;`
  `pub uninterp spec fn spec_entry_from_raw<E>(w: PteWord) -> Option<E>;`
- Added the global ghost model of page-table memory (the analogue of
  `phys_view()`):
  `pub uninterp spec fn spec_table_word(addr: nat, index: nat) -> PteWord;`
  `pub open spec fn spec_table_read<E>(addr, index) -> Option<E> { spec_entry_from_raw::<E>(spec_table_word(addr, index)) }`
- Restored the `view_design.md` target entries-map shape:
  `pub struct TableView<E> { pub addr: nat, pub entries: Map<nat, Option<E>> }`
  and `impl<E: TableEntry> View for Table<E> { type V = TableView<E>; ... }`
  with `entries` defined pointwise from `spec_table_read` over
  `[0, PAGE_TABLE_LENGTH)`. (`from_address`'s `result@.addr == base` is
  unaffected — same `.addr` field.)

**`src/libs/arch/src/x86/mem/paging/table.proof.rs`** (was empty)
- Added the `TableEntry` round-trip law as a broadcast lemma (signature only,
  `admit()` body per the specification phase):
  `pub broadcast proof fn lemma_entry_roundtrip<E>(e: E) ensures #[trigger] spec_entry_from_raw::<E>(spec_entry_raw(e)) == Some(e);`

**`src/libs/arch/src/x86/mem/paging/table.rs`**
- `TableEntry::from_raw` (line ~33) → `ensures result == spec_entry_from_raw::<Self>(raw)`.
- `TableEntry::raw` (line ~38) → `ensures result == spec_entry_raw(self)`.
- `Table::<E>::read` (line ~203, **kept `external_body`**) →
  `requires index@ < crate::mem::PAGE_TABLE_LENGTH`,
  `ensures result == spec_table_read::<E>(self@.addr, index@)`.
- `Table::<E>::write` (line ~232, **kept `external_body`**) →
  `requires index@ < crate::mem::PAGE_TABLE_LENGTH`,
  `ensures spec_table_word(self@.addr, index@) == spec_entry_raw(entry)`.
- No signature changed; no tracked/`with` permission parameter added.

**Why this satisfies the caller analysis**
- `read` returns `Some(e)` for a valid decode / `None` for an invalid word
  (`spec_table_read = spec_entry_from_raw(...)`) → the `InvalidArgument` path.
- Read-after-write round-trip is now caller-derivable: after `write(idx, e)` the
  caller knows `spec_table_word(addr, idx) == spec_entry_raw(e)`, so
  `read(idx) == spec_entry_from_raw(spec_entry_raw(e)) == Some(e)` via
  `broadcast use lemma_entry_roundtrip` (verified with a client proof in a
  scratch crate before applying).
- **Frame condition is free:** because `spec_table_word` is a pure
  (parameter-free) function, a `write` call only updates knowledge of the one
  named slot; every other slot/page is automatically preserved across the call.
  No `old()` and no signature change needed — exactly the `phys_view()` device.
  The cross-call transition / same-slot consistency is realized in the proving
  phase by a ghost token (the documented `phys_view()` placeholder rationale).

Supporting docs updated to reflect the now-contracted boundary: `tcb-allowed.md`,
`bugs.md`, `verus-unsupported.md`, and `view_design.md` (new "Revision (Turn 1
review)" section).

### Verification

**`make verify-arch`** → exit 0:
```
=== Results ===
  23 verified
  0 errors
  Exit code : 0
cheating: assume=0 external_body=2 admit=1 trusted=0 no_decreases=0 cfg_gate=0
coverage: 9/525 exec functions have contracts
```
(`external_body=2` = `read`/`write`, now contracted and TCB-listed; `admit=1` =
`lemma_entry_roundtrip`'s proof body, which the task permits during the
specification phase.)

**`make verify`** (full regression — confirms no cascade into `identity_map`) →
exit 0:
```
arch:   verification results:: 23 verified, 0 errors
kernel: verification results:: 76 verified, 0 errors
```
The `requires index@ < PAGE_TABLE_LENGTH` on `read`/`write` did **not** cascade —
the out-of-scope callers (`ensure_pt`/`ensure_pte`/`identity_map_page`) compile
unchanged because they begin with `proof! { admit(); }` and no ghost parameter
was added.

**`./z build`** (dual compilation) → `[OK] Build complete.` (exit 0).

**Reviewer scripts**
- `fn_coverage.py table.rs table.rs` → 7 matched, 0 missing
  (`from_address, into_raw, new, pd_index, pt_index, read, write`).
- `spec_drift.py check nanvix-phys-arch-paging-table` → **0 contract drift**
  (ensures removed: 0, requires added: 0) — guarantees were only *added*.
- `grep verus_spec table.rs` → `from_raw`, `raw`, `read`, `write` each now carry
  a `#[verus_spec(... ensures ...)]`.

### Result: FIXED
