# View Design: `arch::x86::mem::paging::table`

## Abstract Resource

To callers this module is **a single hardware page-table page**: a fixed-length
array of `PAGE_TABLE_LENGTH` (= 1024) typed entries, addressed by a *validated*
in-range index (`TableIndex`) and backed by volatile, caller-owned memory at a
physical/identity-mapped base address.

Abstractly it is a **partial map `index → Option<E>`** over one page, plus two
pure helpers (`pd_index`, `pt_index`) that extract the directory/table index of
a virtual address. The phantom type `E` keeps PD and PT pages from being mixed.

Two types need abstraction:

| Type | What it is to a caller |
|------|------------------------|
| `TableIndex` | a *validated* slot number, guaranteed `< PAGE_TABLE_LENGTH` |
| `Table<E>` | a typed, non-owning handle whose observable content is the per-slot entry map |

---

## View Structs

### `TableIndex`

A `TableIndex` carries exactly one piece of caller-visible information: the
validated index value. Its abstract value is a single `nat`.

```rust
impl View for TableIndex {
    type V = nat;
    // value returned by into_raw(); always < PAGE_TABLE_LENGTH (see inv)
    pub closed spec fn view(&self) -> nat;
}
```

### `Table<E>`

```rust
pub struct TableView<E: TableEntry> {
    /// Physical/identity-mapped base address this handle denotes.
    /// Caller-meaningful identity: which page read/write act on.
    pub addr: nat,
    /// Logical contents: the entry a `read` would return for each valid slot.
    /// `entries[i] == None` means slot `i` holds a word that is not a valid
    /// encoding of `E`; `Some(e)` (including a not-present, zeroed entry) is a
    /// valid decode.
    pub entries: Map<nat, Option<E>>,
}

impl View for Table<E> {
    type V = TableView<E>;
    pub closed spec fn view(&self) -> TableView<E>;
}
```

`E: TableEntry` is `Copy`; the trait supplies two pure spec projections used by
the transitions below (their `requires`/`ensures` are designed in the spec
phase; `raw`/`from_raw` are in verification scope, so these are *proven*, not
assumed):

```rust
spec fn spec_raw(self: E) -> PteWord;              // serialize
spec fn spec_from_raw(w: PteWord) -> Option<E>;    // decode (None = invalid)
```

with the round-trip **trait law** `spec_from_raw(e.spec_raw()) == Some(e)`.

---

## Well-formedness Invariants

```rust
// TableIndex: the validated-range guarantee callers depend on.
pub open spec fn inv(&self) -> bool {     // on TableIndex
    self@ < PAGE_TABLE_LENGTH
}
```

```rust
impl<E: TableEntry> TableView<E> {
    pub open spec fn inv(self) -> bool {
        // exactly one page worth of slots, no more, no fewer
        &&& self.entries.dom() =~= Set::new(|i: nat| i < PAGE_TABLE_LENGTH)
        // a page-table page is page-aligned
        &&& self.addr % PAGE_SIZE == 0
    }
}
```

The `Table` type-level `inv()` is `self@.inv()`.

---

## Spec Transition Functions

### Pure index extractors (produce a `TableIndex` value = `nat`)

```rust
pub open spec fn spec_table_index(vaddr: nat, shift: nat) -> nat {
    (vaddr >> shift) & (PAGE_TABLE_LENGTH - 1)        // low 10 bits after shift
}
pub open spec fn spec_pd_index(vaddr: nat) -> nat { spec_table_index(vaddr, PGTAB_SHIFT) }
pub open spec fn spec_pt_index(vaddr: nat) -> nat { spec_table_index(vaddr, PAGE_SHIFT) }
```

- `pd_index(vaddr)` → `result@ == spec_pd_index(vaddr as nat)` (and `inv`, i.e.
  `result@ < PAGE_TABLE_LENGTH`, which follows from the mask).
- `pt_index(vaddr)` → `result@ == spec_pt_index(vaddr as nat)` (and `inv`).
- `TableIndex::into_raw(self)` → `result == self@` (identity projection).
- `TableIndex::new(i)` → `Some(t)` iff `i < PAGE_TABLE_LENGTH`, and then
  `t@ == i`; `None` otherwise. *(out of stated scope, but specifiable.)*

### Handle creation

```rust
// from_address only fixes identity; contents are pre-existing memory.
pub open spec fn spec_from_address(base: nat) -> /* addr field of */ nat { base }
```

- `Table::from_address(base)` → `result@.addr == base`. `entries` are left
  unconstrained: they reflect whatever the backing page already holds.

### Read / write (the `index → Option<E>` map)

```rust
impl<E: TableEntry> TableView<E> {
    pub open spec fn spec_read(self, idx: nat) -> Option<E> {
        self.entries[idx]
    }
    pub open spec fn spec_write(self, idx: nat, entry: E) -> TableView<E> {
        TableView {
            entries: self.entries.insert(idx, E::spec_from_raw(entry.spec_raw())),
            ..self                              // frame: addr unchanged
        }
    }
}
```

- `Table::read(&self, index)` → `result == self@.spec_read(index@)`
  (requires `index@ < PAGE_TABLE_LENGTH`, met by `TableIndex::inv`).
- `Table::write(&self, index, entry)` →
  `self@ == old(self)@.spec_write(index@, entry)` — only slot `index@` changes,
  `addr` and every other slot are preserved by `..self` / `insert`.

**Read-after-write** falls out directly:
`spec_read(spec_write(s, i, e), i) == spec_from_raw(e.spec_raw())`, and by the
trait round-trip law this is `Some(e)`. Writes to `j != i` leave slot `i`
untouched because `Map::insert` only alters the keyed slot.

---

## Design Rationale (per field, with substitution test)

> Substitution test: *if the implementation were rewritten with a different
> algorithm, would this field still make sense?*

### `TableIndex@ : nat`
- **Why:** the only thing every caller asks of a `TableIndex` is its numeric
  value and the promise it is `< PAGE_TABLE_LENGTH`. `gva.rs` does
  `into_raw().checked_mul(entry_size)` relying on that bound; `identity_map.rs`
  feeds it straight to `read`/`write`.
- **Substitution:** ✅ Any representation (newtype over `usize`, a `u16`, a
  bitfield slice) still denotes one index value. `nat` is the algorithm-free
  essence; `into_raw` is its identity projection.

### `TableView::addr : nat`
- **Why:** `from_address` has no other observable contract than *“this handle
  denotes the page at `base`.”* Callers build PD/PT handles from physical
  addresses they computed and must know the handle refers to that page.
- **Substitution:** ✅ Any handle to a page-table page must know which page it
  addresses, regardless of how it stores it. (The `PhantomData` marker, by
  contrast, is pure implementation and is excluded — see Rejected.)

### `TableView::entries : Map<nat, Option<E>>`
- **Why:** the read/write round-trip is the central thing callers reason about
  (caller analysis: *“model a table as a finite map `TableIndex → Option<E>`”*).
  `Option<E>` is exactly the read result: `None` = structurally invalid word
  (→ `InvalidArgument` at the call site), `Some(e)` = a valid entry, *including*
  a zeroed not-present entry.
- **Substitution:** ✅ Whether the page is read volatilely word-by-word, via DMA,
  or copied wholesale, its observable state is still “what entry sits in each
  slot.” The map domain `[0, PAGE_TABLE_LENGTH)` is a hardware fact, not an
  implementation choice.
- **Why `Option<E>` and not raw `PteWord`:** callers never reason in raw words;
  they consume `Option<E>`. Storing the decoded value keeps `spec_read` a
  trivial lookup (directly usable in caller proofs) and confines the
  `raw`/`from_raw` codec to a single place: `spec_write`.

### Why decode at *write* time in `spec_write`
- Memory is static between a write and a later read, so decoding may be modeled
  at either end. Decoding at write (`from_raw(entry.raw())`) makes `spec_read` a
  pure map lookup and exposes the entry-codec round-trip as one clean obligation
  on the `E` trait — matching the documented trait expectation — rather than
  smearing `from_raw` across every read site.

---

## Rejected Alternatives

- **Mirror the struct: `TableView { base: usize, _marker: PhantomData<E> }`.**
  Rejected — fails substitution and the caller-only test. `base`-as-`usize` and
  `PhantomData` are *how* the handle is built, not *what* callers observe; this
  View cannot express the read/write round-trip the identity map depends on
  (precisely the weakness called out for the current opaque placeholder).

- **`entries : Map<nat, PteWord>` (raw words instead of decoded).**
  Rejected as the primary shape — faithful but forces `spec_read` to carry a
  `from_raw` on every lookup and pushes raw-word reasoning onto callers who only
  care about `Option<E>`. Kept in mind as an equivalent lower-level model; the
  decoded map is strictly friendlier for caller proofs.

- **`entries : Seq<Option<E>>` (length-1024 sequence).**
  Rejected — a `Seq` ties the abstraction to dense 0..1024 indexing and makes
  per-slot frame conditions noisier (`forall j != i`). `Map` + `insert` gives a
  one-line frame condition and matches the “partial map” mental model.

- **`spec_write` stores `Some(entry)` directly.**
  Rejected — unfaithful for any `E` whose encoding is lossy; it would silently
  assume the round-trip law instead of letting it discharge the read-back. The
  `from_raw(entry.raw())` form is faithful unconditionally and reduces to
  `Some(entry)` *only* via the explicit trait law.

- **Encode the volatile-mechanism / `<< PTE_WORD_SIZE_LOG2` offset in the View.**
  Rejected — pure HOW. Callers explicitly don’t care (caller analysis); the
  byte offset is an addressing detail below the abstraction boundary.

- **A separate `addr`-only `Table` View with contents threaded elsewhere.**
  Considered for the volatile-memory issue below; rejected for the *shape*
  because the deliverable’s transitions (`read`/`write`) need the contents map
  to live in one abstraction. The realization concern is handled at the spec
  layer (next note), not by weakening the View.

---

## Open Mechanism Note (for the spec phase, not a View-shape decision)

`read`/`write` take `&self`, yet `write` must change `self@.entries`. The
contents live in volatile, caller-owned memory, not inside the `Table` struct
(which holds only `base`), so `Table::view()` cannot be a pure function of the
struct alone, and two handles with equal `base` alias the same page.

This does **not** change the View *shape*; it is a realization detail: the
`entries` map will be carried by a ghost memory-permission token (PointsTo-style)
threaded through `read`/`write` at the spec layer, keyed by `addr`. Recorded
here so the specification phase wires `self@.entries` to that permission rather
than to the struct fields. `addr` is what links a handle to its permission.
