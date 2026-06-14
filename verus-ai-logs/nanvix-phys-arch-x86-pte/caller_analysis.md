# Caller Analysis: `arch::x86::mem::paging::pte`

## Script Output

Source file: `src/libs/arch/src/x86/mem/paging/pte.rs`
Script: `scripts/find_callers_lsp.py` (rust-analyzer LSP), crate `arch`.

The LSP run reported **0 external callers** for *every* public function. This is a
**false negative**: rust-analyzer resolved references only within the `arch` crate
and did not index the downstream `kernel` crate references during this run. Manual
`grep` across the workspace confirms the in-scope functions are heavily used by the
`kernel` crate. The findings below are based on direct source inspection of the real
call sites, which supersede the script's empty result.

Crates that depend on `arch` (script header): `sysalloc`, `syscall`, `mkramfs`,
`vfsd`, `kernel`, `uservm`, `arch-rust`, `test-kernel`, `test-mmio-fault`, `testd`.
All in-scope call sites are in `kernel`.

## Scope

Verification-order target functions (only these are in scope):

- `PageTableEntry::new`
- `PageTableEntryFlags::new`
- `PageTableEntry::is_present`
- `PageTableEntryFlags::is_present`

## Real Call Sites (in-scope functions)

### `PageTableEntryFlags::new` (constructs the flag set)
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:118` — `PageTable::map` (present=Present, RW/US/cache from args).
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.rs:192` — `PageTable::unmap` (present=NotPresent, all "off" flags).
- `src/kernel/src/mm/virt/identity_map.rs:653` — `ensure_pte` (present=Present, RW, Supervisor).
- `src/kernel/src/mm/virt/boot_init.rs:208` — passed to `PageTable::fill` (present=Present, RW from region perms).

### `PageTableEntry::new` (constructs the entry from flags + frame)
- `page_table.rs:117` — `map` (flags + `paddr.into_frame_number()`).
- `page_table.rs:191` — `unmap` ("not present" flags + frame).
- `page_table.rs:494` — `replace_cow_frame` (flags cloned from existing PTE via `pte.flags()`, then mutated `set_read_write`/`set_cow`, + new frame).
- `page_table.rs:594` — `fill` (caller-supplied `pte_flags` + computed `frame`).
- `identity_map.rs:652` — `ensure_pte` (flags + frame).

### `PageTableEntry::is_present` (queries the entry's present bit)
- `page_table.rs:106` — `map`: if present → entry is busy (`ResourceBusy`).
- `page_table.rs:181` — `unmap`: if NOT present → "page is not present" error.
- `page_table.rs:272` — `is_mapped`: returns `pte.is_present()` directly.
- `page_table.rs:635`, `:660` — table iteration: `Some(pte) if pte.is_present()` filters live entries.
- `identity_map.rs:640` — `ensure_pte`: if present → no-op return `Ok(())`.

### `PageTableEntryFlags::is_present` (queries the flag set's present bit)
- `page_table.rs:564` — `fill`: validates `if !pte_flags.is_present()` → reject with `InvalidArgument` before bulk-filling entries.

## Trait Obligations

- Trait: `arch::mem::paging::TableEntry` (impl for `PageTableEntry`) — `from_raw`/`raw`
  must be exact inverses over the raw `PteWord` so `Table::read`/`Table::write` round-trip
  page-table memory. **Not in scope** here, but constrains `new`: an entry built via `new`
  and serialized with `raw()` (`into_raw_value`) must, when read back via `from_raw`, expose
  the same `is_present`/`frame_number`/flags. Callers (`write_pte`/`read_pte`,
  `fill` at `:595`) rely on this round-trip.

## Caller Expectations

### `PageTableEntryFlags::new`
- Callers assume: the returned flag set reflects **exactly** the seven flag arguments
  passed; in particular `result.is_present() == (present == PresentFlag::Present)`.
  The OS-defined copy-on-write bit is **not** a parameter — callers rely on it defaulting
  to `NotCopyOnWrite`.
- Callers assume: total/infallible construction (no error path, no panics).
- Callers don't care about: the raw bit layout, ordering of fields, or how individual
  flag enums are stored — they only pass strongly-typed flag enums and later query via
  `is_present`/`is_writable`/`is_cow` or serialize via `into_raw_value`.

### `PageTableEntry::new`
- Callers assume: `result` stores the given `flags` and `frame` faithfully, so that
  `result.is_present() == flags.is_present()`, `result.frame_number() == frame`, and
  `result.flags()` returns an equivalent flag set (used by `replace_cow_frame` which
  reads back `pte.flags()`).
- Callers assume: infallible construction; the entry can be immediately serialized with
  `into_raw_value()`/`raw()` and written to a page table.
- Callers don't care about: internal representation or any packing of frame+flags into a
  word — only that the projection accessors (`is_present`, `frame_number`,
  `frame_address`, `flags`) agree with the constructor inputs.

### `PageTableEntry::is_present`
- Callers assume: returns `true` iff the entry was built/decoded with the present bit set,
  i.e. it mirrors the present flag of `self.flags()`. Control flow depends on it being a
  pure, side-effect-free boolean query (map treats `true` as "busy"; unmap/ensure_pte
  treat `false` as "absent").
- Callers don't care about: any other flag or the frame value when checking presence.

### `PageTableEntryFlags::is_present`
- Callers assume: returns `true` iff the flag set was constructed with
  `PresentFlag::Present`. `fill` uses it as a precondition guard, rejecting flag sets
  whose present bit is clear before writing any entry.
- Callers don't care about: the remaining flags during this specific check.

## Abstract Resource

This module manages a **single x86 page-table entry** — the mapping of one virtual page
to a physical frame number together with its permission/state flags (present, read/write,
user/supervisor, write-through, cache-disable, accessed, dirty, and an OS-defined
copy-on-write bit) — and, separately, the **flag-set** sub-component. Callers treat an
entry as an immutable-by-construction value object: build it from typed flags + a frame,
query its present bit / frame, optionally clone-and-mutate flags, then (de)serialize it to
the raw word stored in hardware page-table memory.

## Key Invariants (caller perspective)

- **Constructor faithfulness:** `PageTableEntry::new(flags, frame)` yields `e` with
  `e.is_present() == flags.is_present()`, `e.frame_number() == frame`, and
  `e.flags()` equivalent to `flags`.
- **Flag faithfulness:** `PageTableEntryFlags::new(present, rw, us, pwt, pcd, a, d)` yields
  `f` with `f.is_present() == (present == Present)` and `cow == NotCopyOnWrite`.
- **Presence delegation:** `PageTableEntry::is_present()` == `self.flags().is_present()`.
- **Purity & totality:** all four functions are total, deterministic, side-effect-free;
  the two constructors never fail.
- **Round-trip consistency (via `TableEntry`):** an entry from `new` serialized with
  `into_raw_value`/`raw` and read back with `from_raw` preserves `is_present`,
  `frame_number`, and flags (boundary obligation; `from_raw_value`/`into_raw_value`
  themselves are out of scope).

## Pre-existing Specs (from upstream verification)

- Source: added during verification of module `kernel::mm::virt::identity_map`
  (file `src/kernel/src/mm/virt/identity_map.spec.rs`).
- In-module spec files `pte.spec.rs` / `pte.proof.rs` exist but are **empty**
  (`verus! { }`). No `#[verus_spec]` annotations and no View type exist on `pte.rs` yet.
- External type specifications: `ExPageTableEntry` / `ExPageTableEntryFlags`
  (`#[verifier::external_type_specification] #[verifier::external_body]`) in
  `identity_map.spec.rs:122–128`.
- `assume_specification` entries present for all four in-scope functions
  (`identity_map.spec.rs:170–185`):
  - `PageTableEntryFlags::new` (L170) — signature only.
  - `PageTableEntry::new` (L180) — signature only.
  - `PageTableEntry::is_present` (L185) — signature only.
  - `PageTableEntryFlags::is_present` — **not** listed (only the entry-level `is_present`
    was needed by `identity_map`; the flag-level one used by `fill` has no upstream spec).

### Assessment
- Coverage: **partial.** Only what `identity_map` needed. `PageTableEntryFlags::is_present`
  (used by `page_table::fill`) is absent.
- Strength: **weak.** Every entry is a bare `assume_specification` with **no
  `requires`/`ensures`** — they assert nothing about the result, so they do not capture the
  constructor-faithfulness or presence-delegation invariants the callers above rely on.
- View design: **does not exist yet** — there is no abstract `View`/`spec` model on
  `PageTableEntry` or `PageTableEntryFlags`. The empty `pte.spec.rs` is the intended home.
  When designing it, model the entry as `(flags, frame)` and the flag set as the eight
  typed flags, exposing `is_present()` as a projection of the present flag — matching the
  caller expectations above rather than the raw-word implementation. These placeholder
  `assume_specification`s should be replaced by real `#[verus_spec]` contracts on `pte.rs`
  once verified (per the documented "placeholders removed when the dependency module is
  verified" methodology already applied to `pde`/the flag enums in this same file).
