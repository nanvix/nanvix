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

---

## As-Built Decision (spec phase) — addr-only struct View, entries deferred

The target design above (per-slot `entries: Map<nat, Option<E>>` with a
`spec_read`/`spec_write` round-trip) requires a ghost memory-permission token
keyed by `addr` (see *Open Mechanism Note*). Implementing that token forces a
`with`-clause ghost parameter onto `read`/`write`, which cascades into their
upstream callers (`identity_map::ensure_pt`/`ensure_pte`/`identity_map_page`)
— all of which are **out of scope** for this module and currently begin their
bodies with `proof! { admit(); }`.

Therefore, in this phase the **struct View of `Table<E>` is `addr`-only**:

```rust
pub struct TableView { pub addr: nat }

impl<E: TableEntry> View for Table<E> {
    type V = TableView;
    closed spec fn view(&self) -> TableView { TableView { addr: self.base as nat } }
}
```

Consequences:

- `from_address(base)` keeps its observable contract: `result@.addr == base`.
- `read`/`write` are **`#[verus_verify(external_body)]` trust boundaries**: their
  bodies materialize a raw pointer from the integer `base` (`usize as *const/*mut`),
  which Verus does not support (see `verus-unsupported.md`), and operate on
  volatile, caller-owned memory. They are recorded in `tcb-allowed.md`.
- The `entries` map, `TableView::inv` (page-alignment + domain), the
  `spec_read`/`spec_write` transitions, and the `TableEntry` round-trip law
  (`spec_from_raw(e.spec_raw()) == Some(e)`) are **deferred** to the future
  permission layer. They lose **no** concrete verification value today because
  no verified caller exercises the round-trip (all callers `admit()`).

What is retained verbatim from the target design:

- `TableIndex@ : nat` with `type_invariant inv = @ < PAGE_TABLE_LENGTH`.
- `into_raw` identity projection (`result as nat == self@`, `result < LEN`).
- `TableIndex::new` Some/None contract on the `< LEN` bound.
- `spec_table_index` / `spec_pd_index` / `spec_pt_index`, and the
  `pd_index`/`pt_index` ensures (`result@ == spec_…` and `result@ < LEN`).

The `entries`-map design is preserved above as the **forward target** for when
the page-table permission layer is verified; at that point `read`/`write` lose
their `external_body` and gain the `spec_read`/`spec_write` postconditions.

---

## Revision (Turn 1 review) — `read`/`write` now carry full contracts

The earlier "addr-only View, entries deferred" decision was **revised** after the
Turn 1 specification review: contract-free `external_body` trust boundaries were
rejected because the precedent they cite (`frame::instance`) is `external_body`
*with* a complete `#[verus_spec]` pinned to a **global, parameter-free** ghost
state (`phys_view()`), adding **no** tracked parameter to the signature.

Following that precedent exactly:

- **Global ghost memory.** `spec_table_word(addr, index) -> PteWord` (uninterp,
  parameter-free) models the raw word at each page-table slot — the analogue of
  `phys_view()`. `spec_table_read::<E>(addr, index) = spec_entry_from_raw(spec_table_word(addr, index))`
  is the decoded entry. Because it is a pure function, a `write` call only
  updates knowledge of the one named slot; **every other slot/page is preserved
  across the call for free** (the caller-facing frame condition), and the
  cross-call transition / same-slot consistency is realized in the proving phase
  by a ghost token (the `phys_view()` placeholder rationale).
- **Entries map restored.** `TableView<E> { addr: nat, entries: Map<nat, Option<E>> }`
  with `entries` defined pointwise from `spec_table_read` over `[0, PAGE_TABLE_LENGTH)`
  — the `Map<nat, Option<E>>` model from the target design above.
- **TableEntry codec + law.** `spec_entry_raw::<E>`/`spec_entry_from_raw::<E>`
  (unbounded over `E` to avoid a trait↔function definitional cycle) abstract
  `raw`/`from_raw`; the round-trip law `spec_entry_from_raw(spec_entry_raw(e)) == Some(e)`
  is the broadcast lemma `lemma_entry_roundtrip` (`table.proof.rs`).
- **Contracts (no signature change).**
  - `from_raw` → `result == spec_entry_from_raw::<Self>(raw)`;
    `raw` → `result == spec_entry_raw(self)`.
  - `read` → `requires index@ < PAGE_TABLE_LENGTH`,
    `ensures result == spec_table_read::<E>(self@.addr, index@)`.
  - `write` → `requires index@ < PAGE_TABLE_LENGTH`,
    `ensures spec_table_word(self@.addr, index@) == spec_entry_raw(entry)`.
- **Read-after-write** is now caller-derivable: after `write(idx, e)`,
  `read(idx) == spec_entry_from_raw(spec_entry_raw(e)) == Some(e)` via
  `broadcast use lemma_entry_roundtrip`.

`read`/`write` remain `#[verus_verify(external_body)]` solely because of the
genuine Verus `usize`→pointer limitation (`verus-unsupported.md`); they are no
longer contract-free. No exec signature changed, so the out-of-scope `admit()`
callers (`identity_map::ensure_pt`/`ensure_pte`/`identity_map_page`) do not
cascade — confirmed by `make verify` (kernel: 76 verified, 0 errors).

---

## Correction (Turn 2 review) — `write` must NOT pin the pure ghost

The Turn 1 revision gave `write` the contents postcondition
`ensures spec_table_word(self@.addr, index@) == spec_entry_raw(entry)`. The Turn 2
review correctly flagged this as **unsound** (#2/#3/#15):

`spec_table_word` is a *pure* `uninterp spec fn` — one fixed value per
`(addr, index)`. Because `write` is `external_body`, the `ensures` is *assumed*
at every call site. Two writes of distinct entries to the same slot then assume
`spec_table_word(a,i) == spec_entry_raw(e1)` **and** `== spec_entry_raw(e2)`, so
`spec_entry_raw(e1) == spec_entry_raw(e2)`; with `lemma_entry_roundtrip` this
gives `Some(e1) == Some(e2)`, i.e. `e1 == e2` — `false` whenever `e1 != e2`.
(Reproduced in a scratch Verus client: `assert(false)` verified.)

**Fix (applied):** `write` keeps only the sound `requires index@ < PAGE_TABLE_LENGTH`
and carries **no** contents `ensures`. The slot-update transition
(`self@.entries[index@] == Some(entry)` after the call, other slots framed) is a
genuine mutable `old@ -> @` state change, which a pure function cannot express;
it is therefore **deferred to the proving-phase page-table permission token** —
the same convention `identity_map.spec.rs` uses to defer `identity_map_view()`'s
`v -> v'`.

Unchanged and sound: `read`'s `ensures result == spec_table_read::<E>(self@.addr,
index@)` (reading a pure accessor is sound — two reads agree), the `raw`/`from_raw`
ensures, `lemma_entry_roundtrip`, and the `TableView<E> { addr, entries }` view.
Reading remains fully specified; only the *write transition* is deferred.
