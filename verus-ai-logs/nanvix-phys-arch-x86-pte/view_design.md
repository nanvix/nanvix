# View Design: `arch::x86::mem::paging::pte` (Page Table Entry)

In-scope (verification-order) targets — the **only** functions this View must
serve, and the only ones any later phase may annotate:

- `PageTableEntryFlags::new`
- `PageTableEntry::new`
- `PageTableEntry::is_present`
- `PageTableEntryFlags::is_present`

All other items (`is_user`, `is_writable`, `is_cow`, `frame_number`,
`frame_address`, the `set_*` mutators, `from_raw_value`/`into_raw_value`,
`flags`, the `SIZE` const, the `TableEntry` impl, derived `Debug`/`Clone`/`Copy`)
are **out of scope** and untouched. Where they are mentioned below it is only to
confirm the View would *also* serve them without redesign.

---

## Abstract Resource

This module models **one x86 32-bit page-table entry**: a value that binds a set
of **paging control flags** to a **physical frame** (the page mapped by the
entry). To a caller a PTE is conceptually the pair

```
( flags , frame )
```

an installable / decodable slot in a page table. From it the in-scope callers
read back exactly one abstract fact: **is it present?** (the present bit, queried
on both the entry and, separately, on a free-standing flags bundle).

Two types need abstraction, mirroring the source structure:

| Type | What it is to a caller |
|------|------------------------|
| `PageTableEntryFlags` | an immutable bundle of the eight paging control bits |
| `PageTableEntry`      | the pair `(flags, frame)` |

It is *not* a collection, allocator, or state machine: both types are immutable,
`Copy`, pure-value tokens. Every in-scope function is either a pure total
constructor (`*::new`) or a pure read-only query (`is_present`); none mutate,
allocate, fail, or panic.

### Downstream contract this View must realize

The verified kernel already pins the *external* contract of these types in
`src/kernel/src/mm/virt/identity_map.spec.rs`:

- the types are lifted opaque via `external_type_specification`
  (`ExPageTableEntry`, `ExPageTableEntryFlags`);
- **placeholder** `assume_specification`s (currently signature-only, no
  `ensures`) stand in for `PageTableEntryFlags::new`, `PageTableEntry::new`, and
  `PageTableEntry::is_present`. `PageTableEntryFlags::is_present` (used by
  `PageTable::fill`) has **no** upstream placeholder yet — this phase introduces
  its first contract.

The View below is chosen so that, once `arch` is verified, the real `ensures`
(the spec transitions in this document) **supersede** those placeholders without
breaking kernel proofs. The frame index reuses the already-shipped `FrameNumber`
abstraction (`frame@ : int`, bounded by `FrameNumber::spec_max()`), exactly as
the sibling `pde` View does.

---

## View Types

### `PageTableEntryFlags`

The abstract value of a flags bundle is exactly its **eight boolean control
bits**. Each source enum is two-valued (`0` = clear, `1 << SHIFT` = set), i.e.
isomorphic to `bool`; the spec-world form of "the bit is set" is a `bool`.

```rust
pub struct PteFlagsView {
    /// Present (P) bit — the entry maps a page (`is_present`).
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
    /// Copy-on-write (OS-defined AVL) bit — set by `set_cow`, read by `is_cow`.
    pub cow: bool,
}

impl View for PageTableEntryFlags {
    type V = PteFlagsView;
    // `closed`: callers reference `self@.present` etc., but the bit-packing into
    // the raw `PteWord` is hidden — exactly the encoding-independence the caller
    // analysis demands.
    closed spec fn view(&self) -> PteFlagsView;
}
```

The single structural difference from the `pde` sibling: a PTE flags bundle
carries an OS-defined **`cow`** bit (an AVL bit) in place of the PDE's
hardware **`large_page` (PS)** bit. This matters for the constructor spec below,
because `cow` is the one bit `new` does **not** take as a parameter.

### `PageTableEntry`

A PTE is the pair `(flags, frame)`. The frame is abstracted as its integer index
(the `FrameNumber` View, `int`); the physical base address it yields
(`frame_address`, out of scope) is *derived* (`frame * FRAME_SIZE`), never
stored.

```rust
pub struct PteView {
    /// The eight control bits this entry was built with.
    pub flags: PteFlagsView,
    /// The frame index this entry points at (== the inner `FrameNumber`'s `@`).
    pub frame: int,
}

impl View for PageTableEntry {
    type V = PteView;
    closed spec fn view(&self) -> PteView;
}
```

---

## Well-formedness Invariants

```rust
impl PageTableEntryFlags {
    pub open spec fn inv(&self) -> bool {
        true   // every combination of the eight bits is a legal flags value
    }
}
```

A flags bundle has no cross-field constraint: all 2⁸ combinations are
constructible (via `new` plus the `set_*` mutators), so its invariant is vacuous.
Kept explicit for uniformity; it adds no obligation. In particular there is **no**
"`cow ⇒ present`" or similar coupling — the hardware/OS imposes none, and `fill`
only ever *checks* `present`, never relates it to `cow`.

```rust
impl PageTableEntry {
    pub open spec fn inv(&self) -> bool {
        // The frame index is a valid frame number, so the derived physical base
        // address `frame * FRAME_SIZE` is well-defined and cannot overflow
        // `usize`. This is exactly the bound `FrameNumber` already guarantees;
        // restating it here lets the (out-of-scope) `frame_address` be total
        // with no `requires`, and keeps `new`'s result well-formed.
        &&& 0 <= self.frame <= FrameNumber::spec_max()
        &&& self.flags.inv()
    }
}
```

The only real constraint is the frame bound, inherited verbatim from the
`FrameNumber` type invariant. No in-scope function depends on it directly (none
of the four reads the frame), but it keeps `new`'s result a well-formed value and
matches the sibling `pde` View so the two compose uniformly in the page-table
layer.

---

## Spec Transition Functions

Both types are immutable; the "transitions" relate constructor inputs / query
self-state to the abstract result. They live on the View types (skill: reusable
spec helpers belong on the View, not as extra `pub spec fn`s on the exec impl).

### Flag projection helpers

Each control enum is two-valued; project the "set" variant to `bool`:

```rust
pub open spec fn spec_present_set(p: PresentFlag)       -> bool { p is Present }
pub open spec fn spec_rw_set(f: ReadWriteFlag)          -> bool { f is ReadWrite }
pub open spec fn spec_us_set(f: UserSupervisorFlag)     -> bool { f is User }
pub open spec fn spec_pwt_set(f: PageWriteThroughFlag)  -> bool { f is WriteThrough }
pub open spec fn spec_pcd_set(f: PageCacheDisableFlag)  -> bool { f is CacheDisabled }
pub open spec fn spec_a_set(f: AccessedFlag)            -> bool { f is Accessed }
pub open spec fn spec_d_set(f: DirtyFlag)               -> bool { f is Dirty }
pub open spec fn spec_cow_set(f: CopyOnWriteFlag)       -> bool { f is CopyOnWrite }
```

### `PageTableEntryFlags::new(present, read_write, …, dirty)` — **7 parameters**

Pure, total. Records each of the **seven** argument bits faithfully, and — the
PTE-specific subtlety — **defaults `cow` to `false`** (`NotCopyOnWrite`), since
`cow` is *not* a parameter (caller invariant: "the OS-defined copy-on-write bit
is not a parameter — callers rely on it defaulting to `NotCopyOnWrite`"):

```rust
pub open spec fn spec_pte_flags_new(
    present: PresentFlag, read_write: ReadWriteFlag, user_supervisor: UserSupervisorFlag,
    page_write_through: PageWriteThroughFlag, page_cache_disable: PageCacheDisableFlag,
    accessed: AccessedFlag, dirty: DirtyFlag,
) -> PteFlagsView {
    PteFlagsView {
        present:        spec_present_set(present),
        writable:       spec_rw_set(read_write),
        user:           spec_us_set(user_supervisor),
        write_through:  spec_pwt_set(page_write_through),
        cache_disabled: spec_pcd_set(page_cache_disable),
        accessed:       spec_a_set(accessed),
        dirty:          spec_d_set(dirty),
        cow:            false,                 // defaulted, not a parameter
    }
}
// new(present, …, dirty)  ensures  result@ == spec_pte_flags_new(present, …, dirty)
```

The `cow: false` clause is load-bearing: it lets a caller (e.g. `unmap`, which
builds an all-"off" flag set) conclude `result.is_cow() == false`, and it makes
the spec reject a buggy `new` that leaks a stale cow bit.

### `PageTableEntryFlags::is_present(&self)`

Pure read-only projection of the present bit (the guard `PageTable::fill` uses):

```rust
// is_present(&self)  ensures  result == self@.present
```

### `PageTableEntry::new(flags, frame)`

Pure, total. Pairs *these exact* flags with *this exact* frame
(constructor fidelity):

```rust
pub open spec fn spec_pte_new(flags: PteFlagsView, frame: int) -> PteView {
    PteView { flags, frame }
}
// new(flags, frame)  ensures  result@ == spec_pte_new(flags@, frame@)
//                             (and result.inv(), since frame.inv() ⇒ frame@ ≤ MAX)
```

From this single fact the callers' derived expectations follow:
`result.is_present() == flags.is_present()`, `result.frame_number() == frame`,
and `result.flags()` is equivalent to `flags` (relied on by `replace_cow_frame`,
which reads back `pte.flags()`).

### `PageTableEntry::is_present(&self)`

Pure read-only; **delegates** to the flags' present bit (presence delegation:
`is_present() == self.flags().is_present()`):

```rust
// is_present(&self)  ensures  result == self@.flags.present
```

`map` treats `true` as "busy" (`ResourceBusy`); `unmap`/`ensure_pte` treat
`false` as "absent". The spec is a side-effect-free boolean, exactly what that
control flow needs.

---

## Design Rationale (per field — substitution test)

> *If the implementation were completely rewritten with a different algorithm,
> would this field still make sense?*

| View element | Why it's needed | Substitution verdict |
|--------------|-----------------|----------------------|
| `PteFlagsView.present` | The bit both in-scope queries observe; drives `is_present` on both types and the `map`/`unmap`/`ensure_pte`/`fill` control flow. | **Passes.** Any encoding still exposes a present bit. |
| `PteFlagsView.cow` | The one bit `new` does **not** take, defaulted to `false`; `new`'s spec must pin it so `is_cow()` is determinate and a stale-bit bug is rejected. Read by `is_cow`, written by `set_cow`/`replace_cow_frame`. | **Passes.** A caller-visible OS attribute, independent of which AVL bit stores it. |
| `PteFlagsView.{writable, user}` | Recorded by `new` (constructor fidelity) and read by sibling accessors `is_writable`/`is_user`; needed so `new`'s `ensures` rejects an impl that drops an argument. | **Passes.** Caller-visible paging permissions, independent of bit layout. |
| `PteFlagsView.{write_through, cache_disabled, accessed, dirty}` | Recorded by `new`; complete the abstract "set of eight control bits" the type *is*, so the constructor spec is total (no flag silently lost). | **Passes.** Architectural control bits, not impl artifacts. |
| `PteView.flags` | A PTE is conceptually `(flags, frame)`; `is_present` delegates here, `flags()` returns it (read back by `replace_cow_frame`). | **Passes.** Any PTE binds control bits to a frame. |
| `PteView.frame : int` | The frame the entry points at; the inverse of `new`'s `frame`; `frame_number`/`frame_address` (out of scope) project it. Reuses `FrameNumber@`. | **Passes.** Every implementation stores *some* frame index; `int` is its representation-free form. |
| `PteView.inv: frame ≤ FrameNumber::spec_max()` | Keeps `new`'s result well-formed and underwrites the out-of-scope `frame_address`'s totality with no `requires`. | **Passes.** The bound is architectural (`MAX_ADDRESS/FRAME_SIZE`), not a code detail. |

**Why eight booleans and not just `present` (+`cow`).** Minimality asks "is every
field used in at least one in-scope spec?" — yes: all eight appear in
`PageTableEntryFlags::new`'s `ensures` (in scope). Dropping the others would make
`new`'s spec one-sided (a buggy `new` that ignores `read_write` would still
verify) and would model only part of the resource the type names ("flags of a
page table entry"). Keeping all eight is therefore both **complete** and
**minimal**, and serves the out-of-scope accessors and later phases with no
redesign.

**Why `cow` is in the View but absent from `new`'s parameter list.** The caller
analysis is explicit: callers rely on `new` defaulting cow to `NotCopyOnWrite`.
Modeling `cow` as a View field lets the constructor spec state `cow == false`
declaratively; without it the spec could not express the default, and `is_cow`
on a freshly-`new`'d entry would be unconstrained.

**Why `frame : int`, not the physical address.** The address is *derived*
(`frame * FRAME_SIZE`); storing it would duplicate state and risk inconsistency
between `frame_number`/`frame_address`. The index is the primitive `new`
consumes — keep one, derive the other. `int` (not `usize`) keeps overflow out of
specs; the bound lives in `inv()`.

**Why composed (`PteView` embeds `PteFlagsView`), not flattened.** The source
binds a *flags value* to a frame, `PageTableEntry::is_present` literally returns
`self.flags().is_present()`, and `flags()` hands the bundle back. Composing the
views makes the "presence delegation" invariant a one-line identity
(`pte@.flags.present`) and keeps the flags abstraction reusable by the standalone
`PageTableEntryFlags::is_present`/`is_cow`/`is_writable` accessors.

All view-design Step-4 checks pass — **Substitution** (table above),
**Caller-only** (present + cow + frame + the other six bits are exactly the
caller's mental model; no `PteWord`, no `FrameNumber` internals leak),
**Complete** (constructor fidelity for flags and entry incl. the cow default,
presence delegation, purity/totality all expressible), **Minimal** (every field
used in an in-scope `ensures`), **No code-as-spec** (specs say *what* — "records
each bit", "cow defaults to false", "present delegates to flags" — never *how*
bits are packed).

---

## Rejected Alternatives

- **Flags View = single `present: bool`.** Rejected: makes
  `PageTableEntryFlags::new`'s spec one-sided (cannot reject a `new` that
  mis-records the other six arguments) and models only a sliver of the
  "page-table-entry flags" resource. The extra bits cost nothing in the in-scope
  proofs yet make the constructor spec total and future-proof the sibling
  accessors.

- **Flags View = `{ present, cow }` only.** Rejected for the same one-sidedness
  reason: `new` still takes `read_write`/`user_supervisor`/etc., so omitting them
  from the View lets a buggy `new` drop those arguments undetected.

- **Omitting `cow` from the Flags View** (only the seven `new` parameters).
  Rejected: then `new`'s spec cannot express the caller-relied-upon default
  (`cow == NotCopyOnWrite`), and `is_cow` on a constructed entry would be
  unspecified. The whole point of a faithful constructor spec is to pin every
  observable bit of the result, including the one that is defaulted rather than
  passed.

- **Flags View mirroring the enum fields (`present: PresentFlag`, …).**
  Rejected: drags exec two-variant enums into spec world; `bool` is their
  mathematical form and what `is_present` returns. Pattern-matching is confined
  to the tiny projection helpers.

- **PTE View storing the physical address (`addr: int`) instead of `frame`.**
  Rejected: `addr` is derived from `frame`; storing it duplicates state and
  invites `frame_number`/`frame_address` disagreement. The frame index is the
  single source of truth.

- **PTE View storing the raw `PteWord` / inner `FrameNumber`.** Rejected:
  exactly the "mirroring internal fields" the methodology forbids; it leaks the
  bit encoding the caller analysis says callers must not depend on (encoding
  independence), breaking the `into_raw_value`/`from_raw_value` round-trip's
  freedom to change layout.

- **Flattened PTE View (`{ present: bool, frame: int }`, no embedded flags).**
  Rejected: loses the `flags()` accessor's abstract value and the natural
  delegation identity; `replace_cow_frame` (which clones `pte.flags()`) and later
  phases would have to re-derive the flags bundle.

- **Open `view()` exposing the bit layout.** Rejected: `closed` keeps the
  encoding hidden so callers reason only through these transitions — consistent
  with the sibling `pde`/`FrameNumber`/`Table` views.

- **A non-vacuous flags `inv()` (e.g. "`cow ⇒ present`" or "`writable ⇒
  present`").** Rejected: neither the hardware nor the OS imposes such a coupling,
  `new` and the `set_*` mutators accept every combination, and `unmap` builds an
  all-off (`present == false`, `cow == false`) set while other paths set `cow`
  independently of `present`. Inventing a constraint would reject legal callers.
