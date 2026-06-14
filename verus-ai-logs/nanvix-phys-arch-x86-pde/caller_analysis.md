# Caller Analysis: `x86/mem/paging/pde.rs` (Page Directory Entry)

## Script Output
See: `verus-ai-logs/nanvix-phys-arch-x86-pde/find_callers_output.md`

> **Note on script results:** the LSP script reported **0 external callers** for
> every public function. This is a false negative: rust-analyzer indexed the
> `arch` crate in isolation and did not resolve cross-crate references from the
> `kernel` crate (and others). Manual `grep`/`view` across `src/` confirms there
> are many real callers, all of which live in the **`kernel`** crate. The
> analysis below is based on those verified call sites, not on the script's
> empty external-caller list.

## Scope

Verification-order target functions (only these are in scope):

- `PageDirectoryEntryFlags::new`
- `PageDirectoryEntry::new`
- `PageDirectoryEntry::is_present`
- `PageDirectoryEntryFlags::is_present`
- `PageDirectoryEntry::frame_address`

## Where the callers live

| Caller file | Functions used |
|---|---|
| `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.rs` | `PageDirectoryEntry::new`, `PageDirectoryEntryFlags::new`, `PageDirectoryEntry::is_present`, (`from_raw_value`, `into_raw_value`, `frame_number`) |
| `src/kernel/src/mm/virt/identity_map.rs` | `PageDirectoryEntry::new`, `PageDirectoryEntryFlags::new`, `PageDirectoryEntry::is_present`, `PageDirectoryEntry::frame_address` |
| `src/kernel/src/mm/virt/vmem.rs` | `PageDirectoryEntry::is_present` (via `read_pde`) |
| `src/kernel/src/mm/virt/identity_map.spec.rs` | Verus `assume_specification[…]` for all in-scope functions |
| `src/uservm/src/guest_profiler/gva.rs` | `PageDirectoryEntry::SIZE` (const only — not in scope) |

`PageDirectoryEntryFlags::is_present` has **no direct external caller**; it is
reached only internally, through `PageDirectoryEntry::is_present`, which
delegates `self.flags.is_present()`. It is in scope because its spec underpins
the spec of `PageDirectoryEntry::is_present`.

## Verus Obligation (the primary consumer)

`src/kernel/src/mm/virt/identity_map.spec.rs` declares external specifications
that the kernel's verified code depends on:

```rust
pub assume_specification[ PageDirectoryEntryFlags::new ]( … 8 flag args … ) -> PageDirectoryEntryFlags;
pub assume_specification[ PageDirectoryEntry::new ](flags: PageDirectoryEntryFlags, frame: FrameNumber) -> PageDirectoryEntry;
pub assume_specification[ PageDirectoryEntry::is_present ](pde: &PageDirectoryEntry) -> bool;
pub assume_specification[ PageDirectoryEntry::frame_address ](pde: &PageDirectoryEntry) -> usize;
```

The types are also lifted as opaque (`external_body`) spec types:
`ExPageDirectoryEntry`, `ExPageDirectoryEntryFlags`. This tells us the **View**
should expose the *abstract* properties these specs need (presence, frame
address, the flag values fed into `new`) — not the bit-level encoding.

## Caller Expectations

### `PageDirectoryEntryFlags::new(present, read_write, user_supervisor, page_write_through, page_cache_disable, accessed, dirty, page_size)`
- **Callers assume:** the returned flags value faithfully *records each of the 8
  flag arguments*, so that a subsequent query reflects the inputs. In
  particular, callers that pass `PresentFlag::Present`/`NotPresent` expect
  `is_present()` on the resulting PDE to return `true`/`false` accordingly
  (see `page_directory.rs::map`/`unmap`, `identity_map.rs::ensure_pt`).
- It is a pure, total constructor — never fails, no side effects.
- **Callers don't care about:** the internal bit layout, field ordering, or how
  flags are packed into the raw `PteWord`.

### `PageDirectoryEntry::new(flags, frame)`
- **Callers assume:** the returned PDE pairs *these exact* `flags` with *this
  exact* `frame`. Specifically:
  - `is_present()` of the result == `flags.is_present()` (the present bit they
    passed in).
  - `frame_address()` of the result == `frame.into_raw_value() << FRAME_SHIFT`,
    i.e. the physical base address of `frame`.
- Pure, total constructor (no failure, no side effects).
- **Callers don't care about:** raw encoding; they immediately either store it
  via `write_pde`/`pd.write` (round-tripping through `into_raw_value`) or read
  it back through the accessors.

### `PageDirectoryEntry::is_present(&self)`
- **Callers assume:** returns the present bit exactly as set when the PDE was
  constructed / decoded. Used as a guard with strong control-flow meaning:
  - `map`: present ⇒ slot busy ⇒ `ResourceBusy` error (refuses to overwrite).
  - `unmap`, `vmem.rs` walks, `ensure_pt`: `!present` ⇒ no page table exists ⇒
    allocate / skip / error.
  - `identity_map.rs::verify_*`: compares `kernel_pde.is_present()` against
    `target_pde.is_present()` for structural equality of address spaces.
- Pure, read-only, total (`&self`, returns `bool`, no panic).
- **Callers don't care about:** any other flag, or how presence is stored.

### `PageDirectoryEntryFlags::is_present(&self)`
- **Only caller is internal** (`PageDirectoryEntry::is_present`). Expectation:
  returns `true` iff the `present` field equals `PresentFlag::Present` — i.e.
  exactly the `present` argument given to `PageDirectoryEntryFlags::new`.
- Pure, read-only, total.
- **Don't care about:** other flag fields.

### `PageDirectoryEntry::frame_address(&self)`
- **Callers assume:** returns the *physical base address* of the page frame the
  PDE points at, i.e. `frame_number << FRAME_SHIFT` (page-aligned). Concretely:
  - `ensure_pt`: when the PDE is already present, returns `pde.frame_address()`
    directly as the physical address of the existing page table.
  - `verify_kernel_mappings`: requires
    `kernel_pde.frame_address() == target_pde.frame_address()` for present
    entries to confirm two page directories share the same page tables.
- Pure, read-only, total; result is always frame-aligned (low `FRAME_SHIFT`
  bits are zero).
- **Callers don't care about:** the `FrameNumber` representation or shift
  details — only that the value is the byte address corresponding to the stored
  frame, and that it is the inverse of the `frame` passed to `new`.

## Abstract Resource

This module models a single **x86 32-bit Page Directory Entry**: a value that
binds a set of **paging control flags** to a **physical frame** (the page table,
or a large page, that the entry points to). From the caller's perspective a PDE
is `(present?, …other flags…, frame_address)` — an installable/decodable slot
in a page directory.

## Key Invariants (caller perspective)

1. **Constructor fidelity (flags):** for any flags `f = PageDirectoryEntryFlags::new(present, …)`,
   `f.is_present() == (present == Present)`.
2. **Constructor fidelity (entry):** for `e = PageDirectoryEntry::new(flags, frame)`,
   `e.is_present() == flags.is_present()` **and**
   `e.frame_address() == frame.into_raw_value() << FRAME_SHIFT`.
3. **Presence delegation:** `PageDirectoryEntry::is_present` ⟺
   `PageDirectoryEntryFlags::is_present` of its flags.
4. **Frame alignment:** `frame_address()` is always page-aligned (returns the
   physical base address of the frame; low `FRAME_SHIFT` bits are zero).
5. **Purity/totality:** `is_present` and `frame_address` are pure read-only
   queries; both `new` constructors are pure and total. None panic or mutate
   observable state.
6. **Encoding independence:** callers only ever observe a PDE through these
   accessors (and the `into_raw_value`/`from_raw_value` round-trip used by
   `read_pde`/`write_pde`); the concrete bit layout is not relied upon and may
   change without breaking callers, provided invariants 1–4 hold.
