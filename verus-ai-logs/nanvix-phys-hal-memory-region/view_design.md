# View Design: hal::mem::types::region

Status: **review & refinement** of the View already present in the in-file
`verus! { }` block of `src/kernel/src/hal/mem/types/region.rs`. The existing
`MemoryRegionView` is sound and caller-abstract; this document confirms each
field against the substitution test, proposes a strengthened `inv()`, and
records the spec transition and reusable spec helpers that later phases will
reference.

In-scope verification-order targets:
`TruncatedMemoryRegion::start`, `MemoryRegion::start`,
`TruncatedMemoryRegion::size`, `MemoryRegion::size`. The View, however, is
module-level: it is the shared abstraction for *both* region types and all of
their accessors, not just the four targets.

---

## Abstract Resource

A **memory region** is a contiguous, non-wrapping half-open byte interval of the
address space, `[start, start + size)`, tagged with metadata
(type / permissions / cache policy). `TruncatedMemoryRegion` is the same
interval additionally constrained so both endpoints are page-aligned (`start`
page-aligned, `size` a multiple of the page size), making it directly usable for
frame/page booking.

From the caller's perspective the only state that matters is the **geometry**
`(start, size)` plus the three **metadata tags** that other accessors expose.
Everything else (the human-readable `name`, the wrapping of one struct inside
another for the truncated variant, the physical layout of the fields) is
internal.

---

## View Struct

Both `MemoryRegion<T>` and `TruncatedMemoryRegion<T>` map to the **same** View
type. The truncated wrapper forwards `view()` to its inner region, so a single
abstraction describes both. `T: Address + View<V = int>`, hence addresses are
abstracted to `int`.

```rust
pub struct MemoryRegionView {
    /// First valid byte address of the region (abstract address, in bytes).
    /// Equals the `start` supplied at construction; for the truncated variant
    /// it is page-aligned. This is also the Ord key for both region types.
    pub start: int,
    /// Length of the region in bytes. The interval is [start, start + size).
    /// For the truncated variant it is a multiple of the page size.
    pub size: int,
    /// Classification of the region (Usable / Reserved / Mmio / Bad).
    pub typ: MemoryRegionType,
    /// Access permissions granted over the region.
    pub perm: AccessPermission,
    /// Optional MMIO caching policy (PWT/PCD); `None` for non-MMIO regions.
    pub cache_policy: Option<MmioCachePolicy>,
}
```

`view()` stays `closed` on both impls (callers reference `self@.start` etc. but
the field-to-storage mapping does not leak):

```rust
impl<T: Address + View<V = int>> View for MemoryRegion<T> {
    type V = MemoryRegionView;
    closed spec fn view(&self) -> MemoryRegionView { /* maps the 5 fields */ }
}

impl<T: Address + View<V = int>> View for TruncatedMemoryRegion<T> {
    type V = MemoryRegionView;
    closed spec fn view(&self) -> MemoryRegionView { self.0@ }   // forward
}
```

---

## Well-formedness Invariant

The current in-file `inv()` (defined only on `TruncatedMemoryRegion`) captures
**only** page alignment:

```rust
&&& self@.start % spec_page_size() == 0
&&& self@.size  % spec_page_size() == 0
```

The caller analysis shows two further geometry properties that every constructed
region guarantees and that callers *depend on* for their arithmetic
(`size - 1`, `start + size - 1`, `size / FRAME_SIZE`). These should be made part
of the abstract well-formedness so they are available wherever a region is in
scope. Recommended layering:

```rust
// Shared geometry well-formedness, valid for BOTH region types.
// (Proposed as a reusable helper on the View — see "Spec Helpers" below.)
//   non-empty:  size >= 1                (new rejects size == 0)
//   no-wrap:    start >= 0 && start + size <= usize::MAX as int + 1
//               (the interval fits the address space; start + size - 1 never
//                overflows, so inclusive-end / frame-range math is well-defined)

impl<T: Address + View<V = int>> MemoryRegion<T> {
    pub open spec fn inv(&self) -> bool {
        self@.wf_geometry()
    }
}

impl<T: Address + View<V = int>> TruncatedMemoryRegion<T> {
    pub open spec fn inv(&self) -> bool {
        &&& self@.wf_geometry()                       // size >= 1, no wrap
        &&& self@.start % spec_page_size() == 0       // page-aligned start
        &&& self@.size  % spec_page_size() == 0       // page-multiple size
    }
}
```

Notes:
- The no-wrap bound is stated abstractly against `usize::MAX` (the widest
  address representation). The concrete `T::max_addr()` bound enforced by `new`
  is *stronger*; the View only needs the weaker no-wrap fact because that is all
  caller arithmetic relies on, and it keeps the invariant `T`-agnostic.
- Adding `wf_geometry` to `MemoryRegion::inv` introduces a new `inv` on the base
  type. If introducing it risks proof churn in unlisted functions during this
  phase, the page-alignment-only `TruncatedMemoryRegion::inv` may be kept as-is
  and `wf_geometry` folded in incrementally — the *design* intent is recorded
  here regardless.

---

## Spec Helpers (on the View type)

Per the skill, reusable spec logic lives on the View, not as extra `pub spec
fn`s on the region impls. These encode exactly the derived quantities callers
compute, so later `ensures` can hand them back directly.

```rust
impl MemoryRegionView {
    /// Non-empty, non-wrapping interval within the address space.
    pub open spec fn wf_geometry(self) -> bool {
        &&& self.size >= 1
        &&& self.start >= 0
        &&& self.start + self.size <= usize::MAX as int + 1
    }

    /// Exclusive end of the half-open interval [start, end).
    pub open spec fn spec_end(self) -> int { self.start + self.size }

    /// Inclusive last byte address (callers compute `start + size - 1`).
    pub open spec fn spec_last(self) -> int { self.start + self.size - 1 }

    /// Whether `addr` lies in [start, start + size).
    pub open spec fn contains(self, addr: int) -> bool {
        self.start <= addr < self.start + self.size
    }
}
```

These are optional conveniences for downstream phases; the four target
accessors themselves only need `result@ == self@.start` / `result == self@.size`
style `ensures`. They are listed so spec authors reuse one definition of
"inclusive end" / "contains" instead of re-deriving overflow-prone arithmetic.

---

## Spec Transition Functions

The region types are almost entirely immutable. The **only** state-mutating
exec function in the module is `MemoryRegion::set_cache_policy`, so it gets the
sole transition (it is out of the current four-function scope but belongs to the
View design for completeness):

```rust
impl MemoryRegionView {
    pub open spec fn spec_set_cache_policy(self, policy: MmioCachePolicy) -> MemoryRegionView {
        MemoryRegionView { cache_policy: Some(policy), ..self }   // frame: geometry + tags preserved
    }
}
// ensures self@ == old(self)@.spec_set_cache_policy(policy)
```

`..self` is the frame condition: `start`, `size`, `typ`, `perm` are unchanged.

The constructors (`MemoryRegion::new`, `TruncatedMemoryRegion::new`,
`new_mmio`, `from_memory_region`, `from_virtual_memory_region`) are *creation*,
not `self`-transitions; they will be specified in later phases by stating the
resulting `self@` (e.g. `result@.start == start@`, `result@.size` rounded up to
a page multiple for the truncated path) rather than via a transition on an
existing View. No transition function is owed for them here.

The four in-scope accessors are pure reads — no transition, just `ensures`
linking the return value to the View field:

| Function | Owed ensures (later phase) |
|----------|----------------------------|
| `MemoryRegion::start`            | `result@ == self@.start` |
| `MemoryRegion::size`             | `result == self@.size`   |
| `TruncatedMemoryRegion::start`   | `result@ == self@.start` (and `result@ % spec_page_size() == 0` under `inv`) |
| `TruncatedMemoryRegion::size`    | `result == self@.size`   |

---

## Design Rationale (per field, substitution test)

> Substitution test: *if the implementation were completely rewritten with a
> different algorithm, would this field still make sense?*

| Field | Caller-observable via | Substitution test | Verdict |
|-------|-----------------------|--------------------|---------|
| `start: int` | `start()` (10 ext. sites: frame number, raw value, identity-map base, overlap key, Ord key) | Any region representation must expose a first address; callers read it as a frame/page base and as the ordering key. Independent of how it is stored (`T` vs `PageAligned<T>`). | ✅ keep |
| `size: int` | `size()` (8 ext. sites: `size - 1`, `size / FRAME_SIZE`, region end, identity-map end) | Any representation must expose a byte length; callers do exact frame division and inclusive-end math. Survives any storage/truncation strategy. | ✅ keep |
| `typ: MemoryRegionType` | `typ()` (14 refs) — routing/classification of regions | The classification is intrinsic region state, not an implementation artifact; survives a rewrite. Needed so `typ()` can later be specified. | ✅ keep |
| `perm: AccessPermission` | `perm()` — permissions consumed when mapping | Intrinsic region attribute, independent of layout. | ✅ keep |
| `cache_policy: Option<MmioCachePolicy>` | `cache_policy()` / mutated by `set_cache_policy()` | Intrinsic MMIO attribute and the target of the module's only mutator; the transition spec needs it. | ✅ keep |

Abstract-type choices: `int` for `start`/`size` (not `usize`/`T`) so spec
arithmetic — inclusive end, frame ranges, overlap — is overflow-free and matches
how every caller uses the values (`into_raw_value()` then integer math). `Option`
mirrors the optional cache policy. `typ`/`perm` are small `Copy` enums/structs
carried as-is; abstracting them further would add no caller value.

Shared View for both region types: the truncated variant *is* the same interval
with a stronger invariant, and its `view()` forwards to the inner region. One
View + a stronger `inv()` is simpler than two near-identical View types and lets
callers (e.g. `MmioRegion::base`/`size`, the mmio allocator) reason uniformly.

`view()` closed / `inv()` open: `closed` hides the field→storage mapping (e.g.
that truncated regions wrap `MemoryRegion<PageAligned<T>>`); `open` `inv()`
exposes the page-alignment and geometry facts callers must rely on
(`size / FRAME_SIZE` exact, `start` needs no re-alignment).

---

## Rejected Alternatives

- **`name: String` field** — `name()` is a public accessor, but the caller
  analysis explicitly states callers "don't care about" the name for any
  geometry/booking use, and no in-scope spec references it. Including it fails
  *minimality* (no spec uses it) without aiding any caller. It is display /
  identity metadata; if `name()` is ever brought into scope, add a
  `name: Seq<char>` field then. **Excluded.**

- **`start: T` / `start: PageAligned<T>` (machine/typed address)** — mirrors the
  implementation field and forces callers to unwrap before doing arithmetic.
  Fails the abstract-type rule. The `int` abstraction (via `T::view() == int`)
  is what every caller actually computes on. **Excluded.**

- **`size: usize` / `nat`** — `usize` re-introduces overflow into specs;
  `nat` would block expressing the no-wrap bound against `usize::MAX as int`
  cleanly and complicates `size - 1` reasoning. `int` is the right spec type.
  **Excluded.**

- **`end: int` as a stored field (instead of `size`)** — redundant with
  `start + size`; storing both invites an extra consistency clause in `inv()`.
  Callers want *both* numbers, but only one is primitive state — `end` is a
  derived helper (`spec_end`), not a field. **Excluded as a field; kept as a
  helper.**

- **A separate `TruncatedMemoryRegionView`** — would duplicate all five fields
  to add nothing but the page-alignment facts, which belong in `inv()` not in a
  new struct. The `closed` forwarding `view()` already gives the truncated type
  its own abstraction surface. **Excluded.**

- **`is_truncated: bool` / a tag distinguishing the two region kinds** — the
  Rust type already distinguishes them; encoding it in the shared View would be
  a type-system fact restated as spec state (an anti-pattern) and unused by any
  caller. **Excluded.**

- **`internal_inv()` mirroring private fields** — there is no internal
  bookkeeping beyond the five abstract fields; the only "hidden" structure is
  the wrapper nesting, which is a `closed view()` concern, not an invariant.
  No separate internal invariant is warranted. **Excluded.**

---

## Quality Review

| Criterion | Check |
|-----------|-------|
| **Substitution** | All five fields describe intrinsic region state surviving any rewrite. ✅ |
| **Caller-only** | Every field is exposed by a public accessor and used by real callers (geometry by the four targets; tags by `typ`/`perm`/`cache_policy`). ✅ |
| **Complete** | `(start, size)` + tags cover every caller-observable concept in the analysis (frame math, overlap, Ord key, routing, MMIO policy). ✅ |
| **Minimal** | No unused field — `name` deliberately omitted. Each field maps to ≥1 (current or near-term) spec. ✅ |
| **No code-as-spec** | View states *what* a region is (an interval + tags), not *how* it is stored or truncated. ✅ |
