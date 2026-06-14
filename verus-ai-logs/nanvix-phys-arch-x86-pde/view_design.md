# View Design: `arch::x86::mem::paging::pde` (Page Directory Entry)

In-scope (verification-order) targets — the **only** functions this View must
serve, and the only ones any later phase may annotate:

- `PageDirectoryEntryFlags::new`
- `PageDirectoryEntry::new`
- `PageDirectoryEntry::is_present`
- `PageDirectoryEntryFlags::is_present`
- `PageDirectoryEntry::frame_address`

All other items (`is_user`, `is_writable`, `is_large_page`, the `set_*`
mutators, `from_raw_value`/`into_raw_value`, `flags`, `frame_number`, the `SIZE`
const, the `TableEntry` impl, derived `Debug`/`Clone`/`Copy`) are **out of
scope** and untouched. Where they are mentioned below it is only to confirm the
View would *also* serve them without redesign.

---

## Abstract Resource

This module models **one x86 32-bit page-directory entry**: a value that binds
a set of **paging control flags** to a **physical frame** (the page table — or,
for a large page, the mapped region — that the entry points at). To a caller a
PDE is conceptually the pair

```
( flags , frame )
```

an installable / decodable slot in a page directory. From it callers read back
exactly two abstract facts in scope: **is it present?** and **what physical base
address does it point at?** (`frame << FRAME_SHIFT`).

Two types need abstraction, mirroring the source structure:

| Type | What it is to a caller |
|------|------------------------|
| `PageDirectoryEntryFlags` | an immutable bundle of the eight paging control bits |
| `PageDirectoryEntry`      | the pair `(flags, frame)` |

It is *not* a collection, allocator, or state machine: both types are
immutable, `Copy`, pure-value tokens. Every in-scope function is either a pure
total constructor (`*::new`) or a pure read-only query (`is_present`,
`frame_address`); none mutate, allocate, fail, or panic.

### Downstream contract this View must realize

The verified kernel already pins the *external* contract of these types in
`src/kernel/src/mm/virt/identity_map.spec.rs`:

- the types are lifted opaque via `external_type_specification`
  (`ExPageDirectoryEntry`, `ExPageDirectoryEntryFlags`);
- four **placeholder** `assume_specification`s (currently signature-only, no
  `ensures`) stand in for `PageDirectoryEntryFlags::new`,
  `PageDirectoryEntry::new`, `PageDirectoryEntry::is_present`,
  `PageDirectoryEntry::frame_address`.

The View below is chosen so that, once `arch` is verified, the real `ensures`
(the spec transitions in this document) **supersede** those placeholders without
breaking kernel proofs. The frame index reuses the already-shipped
`FrameNumber` abstraction (`frame@ : int`, bounded by `FrameNumber::spec_max()`)
and the frame/page size reuses the existing `mem::FRAME_SIZE` spec constant —
the same one `FrameNumber::spec_max()` is defined over — so the address
`frame@ * FRAME_SIZE` is the inverse of the `frame` passed to `new`.

---

## View Types

### `PageDirectoryEntryFlags`

The abstract value of a flags bundle is exactly its **eight boolean control
bits**. Each source enum is two-valued (`0` = clear, `1 << SHIFT` = set), i.e.
isomorphic to `bool`; the spec-world form of "the bit is set" is a `bool`.

```rust
pub struct PdeFlagsView {
    /// Present (P) bit — the entry maps something.
    pub present: bool,
    /// Read/Write (R/W) bit — writes permitted (`is_writable`).
    pub writable: bool,
    /// User/Supervisor (U/S) bit — user-mode access permitted (`is_user`).
    pub user: bool,
    /// Page-Write-Through (PWT) bit.
    pub write_through: bool,
    /// Page-Cache-Disable (PCD) bit.
    pub cache_disabled: bool,
    /// Accessed (A) bit.
    pub accessed: bool,
    /// Dirty (D) bit.
    pub dirty: bool,
    /// Page-Size (PS) bit — entry maps a large page (`is_large_page`).
    pub large_page: bool,
}

impl View for PageDirectoryEntryFlags {
    type V = PdeFlagsView;
    // `closed`: callers reference `self@.present` etc., but the bit-packing into
    // the raw `PteWord` is hidden — exactly the encoding-independence the caller
    // analysis demands.
    closed spec fn view(&self) -> PdeFlagsView;
}
```

### `PageDirectoryEntry`

A PDE is the pair `(flags, frame)`. The frame is abstracted as its integer
index (the `FrameNumber` View, `int`); the physical base address it yields is
*derived* (`frame * FRAME_SIZE`), never stored.

```rust
pub struct PdeView {
    /// The eight control bits this entry was built with.
    pub flags: PdeFlagsView,
    /// The frame index this entry points at (== the inner `FrameNumber`'s `@`).
    pub frame: int,
}

impl View for PageDirectoryEntry {
    type V = PdeView;
    closed spec fn view(&self) -> PdeView;
}
```

---

## Well-formedness Invariants

```rust
impl PageDirectoryEntryFlags {
    pub open spec fn inv(&self) -> bool {
        true   // every combination of the eight bits is a legal flags value
    }
}
```

A flags bundle has no cross-field constraint: all 2⁸ combinations are
constructible by `new`, so its invariant is vacuous. (Kept explicit for
uniformity; it adds no obligation.)

```rust
impl PageDirectoryEntry {
    pub open spec fn inv(&self) -> bool {
        // The frame index is a valid frame number, so the derived physical base
        // address `frame * FRAME_SIZE` is well-defined and cannot overflow
        // `usize`. This is exactly the bound `FrameNumber` already guarantees;
        // restating it here lets `frame_address` be total with no `requires`.
        &&& 0 <= self.frame <= FrameNumber::spec_max()
        &&& self.flags.inv()
    }
}
```

The only real constraint is the frame bound, inherited verbatim from the
`FrameNumber` type invariant. It is what makes `frame_address` total and
overflow-free — the single guarantee the caller analysis pins on it.

---

## Spec Transition Functions

Both types are immutable; the "transitions" relate constructor inputs / query
self-state to the abstract result. They live on the View types (skill: reusable
spec helpers belong on the View, not as extra `pub spec fn`s on the exec impl).

### Flag projection helper

Each control enum is two-valued; project the "set" variant to `bool`:

```rust
// One per flag enum, e.g.:
pub open spec fn spec_present_set(p: PresentFlag) -> bool { p is Present }
pub open spec fn spec_rw_set(f: ReadWriteFlag)      -> bool { f is ReadWrite }
pub open spec fn spec_us_set(f: UserSupervisorFlag) -> bool { f is User }
pub open spec fn spec_pwt_set(f: PageWriteThroughFlag) -> bool { f is WriteThrough }
pub open spec fn spec_pcd_set(f: PageCacheDisableFlag) -> bool { f is CacheDisabled }
pub open spec fn spec_a_set(f: AccessedFlag)        -> bool { f is Accessed }
pub open spec fn spec_d_set(f: DirtyFlag)           -> bool { f is Dirty }
pub open spec fn spec_ps_set(f: PageSizeFlag)       -> bool { f is Large }
```

### `PageDirectoryEntryFlags::new(present, read_write, …, page_size)`

Pure, total. Records each of the eight arguments faithfully (caller invariant 1):

```rust
pub open spec fn spec_pde_flags_new(
    present: PresentFlag, read_write: ReadWriteFlag, user_supervisor: UserSupervisorFlag,
    page_write_through: PageWriteThroughFlag, page_cache_disable: PageCacheDisableFlag,
    accessed: AccessedFlag, dirty: DirtyFlag, page_size: PageSizeFlag,
) -> PdeFlagsView {
    PdeFlagsView {
        present:        spec_present_set(present),
        writable:       spec_rw_set(read_write),
        user:           spec_us_set(user_supervisor),
        write_through:  spec_pwt_set(page_write_through),
        cache_disabled: spec_pcd_set(page_cache_disable),
        accessed:       spec_a_set(accessed),
        dirty:          spec_d_set(dirty),
        large_page:     spec_ps_set(page_size),
    }
}
// new(..)  ensures  result@ == spec_pde_flags_new(present, …, page_size)
```

### `PageDirectoryEntryFlags::is_present(&self)`

Pure read-only projection (caller invariant 3, the present field):

```rust
// is_present(&self)  ensures  result == self@.present
```

### `PageDirectoryEntry::new(flags, frame)`

Pure, total. Pairs *these exact* flags with *this exact* frame (caller invariant 2):

```rust
pub open spec fn spec_pde_new(flags: PdeFlagsView, frame: int) -> PdeView {
    PdeView { flags, frame }
}
// new(flags, frame)  ensures  result@ == spec_pde_new(flags@, frame@)
//                             (and result.inv(), since frame.inv() ⇒ frame@ ≤ MAX)
```

From this single fact the caller's two derived expectations follow:
`result.is_present() == flags.is_present()` and
`result.frame_address() == frame@ * FRAME_SIZE`.

### `PageDirectoryEntry::is_present(&self)`

Pure read-only; **delegates** to the flags' present bit (caller invariants 3
and "presence delegation"):

```rust
// is_present(&self)  ensures  result == self@.flags.present
```

### `PageDirectoryEntry::frame_address(&self)`

Pure read-only; the physical base address of the pointed-at frame — the inverse
of the `frame` given to `new`, always frame-aligned (caller invariant 4). `FRAME_SIZE`
is the existing `mem::FRAME_SIZE` spec constant, `= 1 << FRAME_SHIFT`:

```rust
// frame_address(&self)
//   ensures  result as int == self@.frame * (mem::FRAME_SIZE as int)
//            result as int % (mem::FRAME_SIZE as int) == 0          // frame-aligned (derived)
```

The alignment clause is derivable from the product form and need not be a
separate primitive guarantee; it is listed because `verify_kernel_mappings`
compares frame addresses for present entries. Totality / no-overflow rests on
`self.inv()` (`self@.frame ≤ FrameNumber::spec_max()`), so no `requires` is
needed — matching the placeholder's contract-free signature.

---

## Design Rationale (per field — substitution test)

> *If the implementation were completely rewritten with a different algorithm,
> would this field still make sense?*

| View element | Why it's needed | Substitution verdict |
|--------------|-----------------|----------------------|
| `PdeFlagsView.present` | The one bit every in-scope query observes; drives `is_present` on both types and the `map`/`unmap`/`ensure_pt` control flow (present ⇒ busy / table exists). | **Passes.** Any encoding still exposes a present bit. |
| `PdeFlagsView.{writable, user, large_page}` | Recorded by `new` (constructor fidelity) and read by the sibling accessors `is_writable`/`is_user`/`is_large_page`; needed so `new`'s `ensures` rejects a buggy impl that drops an argument. | **Passes.** Each is a caller-visible paging permission/attribute, independent of bit layout. |
| `PdeFlagsView.{write_through, cache_disabled, accessed, dirty}` | Recorded by `new`; complete the abstract "set of eight control bits" that the type *is*, so the constructor spec is total (no flag silently lost). | **Passes.** Architectural control bits, not impl artifacts. |
| `PdeView.flags` | A PDE is conceptually `(flags, frame)`; `is_present` delegates here, `flags()` returns it. | **Passes.** Any PDE binds control bits to a frame. |
| `PdeView.frame : int` | The frame the entry points at; `frame_address` derives the physical base from it; the inverse of `new`'s `frame`. Reuses `FrameNumber@`. | **Passes.** Every implementation stores *some* frame index; `int` is its representation-free form. |
| `PdeView.inv: frame ≤ FrameNumber::spec_max()` | Underwrites totality/overflow-safety of `frame_address` with no `requires`. | **Passes.** The bound is architectural (`MAX_ADDRESS/FRAME_SIZE`), not a code detail. |

**Why eight booleans and not just `present`.** Minimality asks "is every field
used in at least one in-scope spec?" — yes: all eight appear in
`PageDirectoryEntryFlags::new`'s `ensures` (which *is* in scope). Dropping the
other seven would make `new`'s spec one-sided (a buggy `new` that ignores
`read_write` would still verify) and would model only part of the resource the
type names ("flags of a page directory entry"). Keeping all eight is therefore
both **complete** and **minimal**, and serves the out-of-scope accessors and
later phases with no redesign.

**Why `frame : int`, not the physical address.** The address is *derived*
(`frame * FRAME_SIZE`); storing it would duplicate state and risk inconsistency
between `is_present`/`frame_number`/`frame_address`. The index is the primitive
that `new` consumes and `frame_address` reads — keep one, derive the other.
`int` (not `usize`) keeps overflow out of specs; the bound lives in `inv()`.

**Why composed (`PdeView` embeds `PdeFlagsView`), not flattened.** The source
binds a *flags value* to a frame, `PageDirectoryEntry::is_present` literally
calls `self.flags.is_present()`, and `flags()` hands the bundle back. Composing
the views makes the "presence delegation" invariant a one-line identity
(`pde@.flags.present`) and keeps the flags abstraction reusable by the PTE
sibling and the standalone flags accessors.

All view-design Step-4 checks pass — **Substitution** (table above),
**Caller-only** (present + frame address + the eight bits are exactly the
caller's mental model; no `PteWord`, no `FrameNumber` internals leak),
**Complete** (constructor fidelity for flags and entry, presence delegation,
frame-aligned address, purity/totality all expressible), **Minimal** (every
field used in an in-scope `ensures`), **No code-as-spec** (specs say *what* —
"records each bit", "address = frame·FRAME_SIZE" — never *how* bits are packed).

---

## Rejected Alternatives

- **Flags View = single `present: bool`.** Rejected: makes
  `PageDirectoryEntryFlags::new`'s spec one-sided (cannot reject a `new` that
  mis-records the other seven arguments) and models only a sliver of the
  "page-directory-entry flags" resource. The other seven cost nothing in the
  in-scope proofs (only `present` is *consumed* downstream) yet make the
  constructor spec total and future-proof the sibling accessors.
- **Flags View mirroring the enum fields (`present: PresentFlag`, …).**
  Rejected: drags exec two-variant enums into spec world; `bool` is their
  mathematical form and what `is_present` returns. (Pattern-matching is confined
  to the tiny projection helpers.)
- **PDE View storing the physical address (`addr: int`) instead of `frame`.**
  Rejected: `addr` is derived from `frame`; storing it duplicates state and
  invites `is_present`/`frame_address` disagreement. The frame index is the
  single source of truth.
- **PDE View storing the raw `PteWord` / inner `FrameNumber`.** Rejected:
  exactly the "mirroring internal fields" the methodology forbids; it leaks the
  bit encoding the caller analysis says callers must not depend on (encoding
  independence, invariant 6), breaking the `into_raw_value`/`from_raw_value`
  round-trip's freedom to change layout.
- **Flattened PDE View (`{ present: bool, frame: int }`, no embedded flags).**
  Rejected: loses the `flags()` accessor's abstract value and the natural
  delegation identity; would force later phases to re-derive the flags bundle.
- **Open `view()` exposing the bit layout.** Rejected: `closed` keeps the
  encoding hidden so callers reason only through these transitions — consistent
  with the sibling `FrameNumber`/`Table` views.
- **A non-vacuous flags `inv()` (e.g. "`large_page ⇒ present`").** Rejected: the
  hardware imposes no such constraint and `new` accepts every combination;
  inventing one would reject legal callers.
