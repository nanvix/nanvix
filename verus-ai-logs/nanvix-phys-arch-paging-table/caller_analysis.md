# Caller Analysis: `arch::x86::mem::paging::table`

## Script Output

Raw `find_callers_lsp.py` output reported **7 public exec functions, 0 private,
2 types (`Table`, `TableIndex`)** and — importantly — **0 external callers for
every public function** (only internal callers of `TableIndex::into_raw` from
`Table::read`/`Table::write`).

> **Why the script found 0 external callers (false negative).**
> This module lives under `src/libs/arch/src/x86/` — the **32-bit x86** build.
> rust-analyzer (and thus the script) indexes the **x86_64 host** configuration,
> so all real call sites — which are compiled only under `cfg(target_arch =
> "x86")` (kernel) or are host-side software page-walkers — are invisible to the
> LSP. A textual search across the workspace recovers the true callers below.
> Treat the script's "no external callers" warning as a configuration artifact,
> **not** as evidence of dead code.

In-scope (verification-order) functions: `Table::write`, `TableIndex::into_raw`,
`raw`, `Table::read`, `from_raw`, `pt_index`, `TableIndex`, `pd_index`,
`Table::from_address`. (`raw`/`from_raw` are the `TableEntry` trait methods,
implemented in `pde.rs`/`pte.rs` and dispatched only inside `Table::read`/`write`.)

## Real Callers (recovered by source search)

### 1. Kernel identity map — `src/kernel/src/mm/virt/identity_map.rs` (primary consumer)

This is the canonical user of the whole module. It walks/builds the kernel's
two-level identity map:

- `paging::pd_index(vaddr)` / `paging::pt_index(vaddr)` → `TableIndex`
  (L131, L306, L728, L729, L783, L848).
- `unsafe { Table::<PageDirectoryEntry>::from_address(pd_paddr) }` and
  `Table::<PageTableEntry>::from_address(pt_paddr)` to obtain a typed view over a
  physical (identity-mapped) page (L133, L297, L299, L732, L736, L785, L852, L863).
- `unsafe { pd.read(pde_idx) }` / `pt.read(pte_idx)` returning
  `Option<PageDirectoryEntry>` / `Option<PageTableEntry>`; callers treat `None`
  as "invalid entry" → `ErrorCode::InvalidArgument` (L308, L316, L535, L633,
  L786, L853, L864).
- `unsafe { pd.write(pde_idx, new_pde) }` / `pt.write(pte_idx, new_pte)` to
  install freshly-built entries (L320, L583, L665).
- The entry-type parameter `E` is fixed per level: PD tables only carry
  `PageDirectoryEntry`, PT tables only `PageTableEntry`. The phantom-typed
  `Table<E>` is exactly what enforces this at the call sites.

### 2. UserVM guest page walker — `src/uservm/src/guest_profiler/gva.rs` (host-side)

`translate_gva()` performs a *software* two-level walk over guest memory:

```rust
let pd_index: usize = paging::pd_index(gva as usize).into_raw();
let pt_index: usize = paging::pt_index(gva as usize).into_raw();
```

Here only `pd_index`/`pt_index` and `TableIndex::into_raw` are used; the table
bytes are read directly from a host pointer (`read_unaligned`) rather than via
`Table::read`, because the guest's memory is not mapped into the host address
space. This caller relies *solely* on the index-extraction arithmetic being
correct (mask to `[0, PAGE_TABLE_LENGTH)`) and on `into_raw` returning that same
value unchanged.

## Pre-existing Specs (from upstream verification)

- **Source:** added while verifying the kernel `mm::virt::identity_map` module.
- **File:** `src/kernel/src/mm/virt/identity_map.spec.rs`.
- These are **trusted external boundary specs** for this not-yet-verified module:
  - `#[verifier::external_type_specification] #[external_body]` for
    `Table<E>` (`ExTable`) and `TableIndex` (`ExTableIndex`) — both opaque.
  - `#[verifier::external_trait_specification]` for `TableEntry` (`ExTableEntry`,
    `: Copy`).
  - `assume_specification` (no `requires`/`ensures` — state-free placeholders) for
    `pd_index`, `pt_index`, `Table::<E>::from_address`, `Table::<E>::read`,
    `Table::<E>::write`, plus `invlpg`.
- Functions WITHOUT specs: `TableIndex::new`, `TableIndex::into_raw`,
  and the `TableEntry::raw`/`from_raw` trait methods (no external spec; opaque).
- View type: **none** — `Table`/`TableIndex` are currently opaque (`external_body`).

### Assessment

- **Coverage:** partial. Only the functions the identity-map walker needed are
  given boundary specs; `into_raw`/`new` and the `raw`/`from_raw` trait methods
  are unspecified.
- **Strength:** weak — the `assume_specification`s are state-free with no
  `ensures`, and the corresponding obligations in the exec bodies are
  `admit()`-ed (`ensure_pte`, `ensure_pt`, etc.). They exist only to let the
  identity-map bodies translate, not to prove anything about table contents.
- **View design:** absent / biased. There is no abstract `View` yet; `Table` is
  fully opaque. A real View must model a table as a finite map
  `TableIndex → Option<E>` so that `read` after `write` can be reasoned about —
  the current opaque placeholder cannot express the read/write round-trip the
  identity map relies on.

## Trait Obligations

- **Trait `TableEntry: Copy`** — semantics expected by `Table`:
  - `from_raw(raw) -> Option<Self>`: parse a raw `PteWord` (`u32`), returning
    `None` exactly for raw values that are not a valid encoding of `E`.
  - `raw(self) -> PteWord`: serialize back to the raw word.
  - Round-trip expectation from callers: an entry built via `E::new(...)` and
    written, then read back, must yield `Some(equivalent entry)`; an
    all-zero / not-present word read by the identity map is a *valid* entry
    (returns `Some`, with `is_present() == false`) — `None` denotes structural
    corruption, which callers surface as `InvalidArgument`.
  - `Copy` lets `write` take `entry` by value and `read` return it freely.

## Caller Expectations

### `pd_index(vaddr) -> TableIndex`
- Callers assume: returns the PD index = `(vaddr >> PGTAB_SHIFT) &
  (PAGE_TABLE_LENGTH - 1)`, always in `[0, PAGE_TABLE_LENGTH)`. Total (no panic,
  no failure) for any `usize`. `const`.
- Callers don't care about: how the bits are masked, or that the result wraps an
  internal `usize` — only that `into_raw()` of it equals the masked value.

### `pt_index(vaddr) -> TableIndex`
- Callers assume: returns the PT index = `(vaddr >> PAGE_SHIFT) &
  (PAGE_TABLE_LENGTH - 1)`, always in range; total; `const`.
- Callers don't care about: internal representation.

### `TableIndex` (type) / `TableIndex::into_raw(self) -> usize`
- Callers assume: a `TableIndex` is a *validated* index, guaranteed
  `< PAGE_TABLE_LENGTH`; `into_raw` is the identity projection back to that
  validated `usize` (loss-less, total, `const`). `gva.rs` multiplies it by entry
  size with `checked_mul`, relying on the bound to stay within a page.
- Callers don't care about: that it is a newtype over `usize`; only the in-range
  invariant and the round-trip with the index extractors matter.

### `Table::from_address(base) -> Table<E>` (`unsafe`, `const`)
- Callers assume: produces a typed, non-owning handle over the page at physical/
  identity-mapped `base`; cheap, infallible, side-effect-free. The caller carries
  the safety obligation (`base` page-aligned, mapped, readable/writable).
- Callers don't care about: that the struct stores only `base` + a `PhantomData`
  marker; they care that the phantom type `E` is threaded so PD/PT handles can't
  be confused.

### `Table::read(&self, index) -> Option<E>` (`unsafe`)
- Callers assume: volatile-reads the word at `base + index*4` and decodes it via
  `E::from_raw`; `Some(e)` for a valid encoding, `None` for an invalid one. Does
  not mutate the table. The validated `index` guarantees in-bounds access.
- Callers don't care about: the volatile-read mechanism or the `<<
  PTE_WORD_SIZE_LOG2` offset math — only the index→entry mapping and the `None`
  = "invalid entry" contract (which they map to `InvalidArgument`).

### `Table::write(&self, index, entry)` (`unsafe`)
- Callers assume: volatile-writes `entry.raw()` to `base + index*4`, so that a
  subsequent `read(index)` observes `Some(entry)` (read-after-write round-trip).
  Only the targeted slot changes. The validated `index` keeps the write in
  bounds of the page.
- Callers don't care about: the volatile mechanism or offset computation.

### `TableEntry::from_raw` / `TableEntry::raw`
- No direct external callers; invoked only inside `read`/`write`. Expectation is
  the round-trip described under *Trait Obligations*.

## Abstract Resource

This module manages a **single hardware page-table page** as a fixed-length array
of `PAGE_TABLE_LENGTH` typed entries, addressed by a *validated* in-range index
(`TableIndex`) and accessed by raw base address. Abstractly it is a partial map
`TableIndex → Option<E>` over volatile memory the caller owns, plus the pure
helpers that derive the PD/PT index of a virtual address.

## Key Invariants (caller perspective)

- **Index validity:** every `TableIndex` (whether from `pd_index`, `pt_index`, or
  `new`) satisfies `into_raw() < PAGE_TABLE_LENGTH`; this bound is what keeps
  `read`/`write` (and `gva.rs`'s `checked_mul`) within one page.
- **Index extraction correctness:** `pd_index`/`pt_index` return exactly the
  masked/shifted bit-field of `vaddr`, and `into_raw` returns that same value.
- **Read/write round-trip:** `write(i, e)` followed by `read(i)` yields
  `Some(e')` with `e'` decoding-equivalent to `e`; writes touch only slot `i`.
- **Decode totality:** `read` returns `None` only for raw words that are not a
  valid `E` encoding; a not-present (zeroed) entry is a valid `Some`, not `None`.
- **Type discipline:** `Table<E>`'s phantom `E` prevents mixing PD and PT entries
  across table levels — a property callers depend on but that has no runtime cost.
- **Non-ownership:** `from_address` does not allocate or free; lifetime/validity
  of the backing page is the caller's responsibility (encoded as `unsafe`).
