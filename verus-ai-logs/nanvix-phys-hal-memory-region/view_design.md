# View Design: hal-memory-region (`region.rs`)

## Abstract Resource

To a caller, a memory region is an **immutable, contiguous, half-open address
range `[start, start + size)`** in a typed address space (`T: Address`), tagged
with descriptive metadata (type, access permission, optional MMIO cache policy).
`MemoryRegion<T>` is the general form; `TruncatedMemoryRegion<T>` is the *same*
resource with both endpoints snapped to page granularity. The entire geometry a
caller reasons about is the pair `(start, size)`; everything else is metadata
exposed through plain getters.

Verification-scope functions (the only exec functions in scope here):
`MemoryRegion::{start,size}`, `TruncatedMemoryRegion::{start,size}` — pure,
deterministic getters that project the abstract geometry.

---

## View Struct

A **single** View type is shared by both region kinds (the truncated region is a
newtype wrapper `TruncatedMemoryRegion(MemoryRegion<PageAligned<T>>)`, so its
abstract state is exactly the inner region's). This keeps the abstraction
boundary uniform and lets the delegating truncated getters relate trivially to
the inner geometry.

```rust
pub struct MemoryRegionView {
    /// Numeric base address of the region (the raw value of `start`,
    /// i.e. `start.into_raw_value() as int`). This is the geometry's low
    /// endpoint and the sole ordering key for regions.
    pub start: int,
    /// Byte length of the region. The range covered is `[start, start + size)`.
    pub size: int,
    /// Region classification (Usable / Reserved / Mmio / Bad).
    pub typ: MemoryRegionType,
    /// Access permission attached to the region.
    pub perm: AccessPermission,
    /// Optional MMIO caching policy (PWT/PCD control); `None` for non-MMIO.
    pub cache_policy: Option<MmioCachePolicy>,
}
```

`start` is a mathematical `int` rather than `T` because every caller projects it
to a raw numeric value (`into_raw_value`, `into_frame_number`) or compares it for
ordering, and the `Address` trait already fixes `T: View<V = int>` with
`into_raw_value() as int == self@`. Working in `int` keeps the geometry
overflow-free and matches the address abstraction. `size` is `int` for the same
reason — it is added to/compared with `start` in caller overlap and frame math.

### Reusable spec helpers (placed on the View, not on `impl MyType`)

```rust
impl MemoryRegionView {
    /// Geometry well-formedness shared by every region kind: the range is
    /// non-empty. (Every constructor rejects `size == 0`.)
    pub open spec fn wf(self) -> bool {
        self.size > 0
    }

    /// Page-granular geometry: both endpoints sit on a page boundary. Only the
    /// truncated kind guarantees this.
    pub open spec fn is_page_aligned(self) -> bool {
        &&& self.start % spec_page_size() == 0
        &&& self.size  % spec_page_size() == 0
    }
}
```

---

## Well-formedness Invariant

The alignment guarantee distinguishes the two kinds, so `inv()` is defined
per type (an untruncated `MemoryRegion` carries **no** alignment promise).

```rust
impl<T: Address + View<V = int>> MemoryRegion<T> {
    pub open spec fn inv(&self) -> bool {
        // Non-empty geometry. (In-range `start + size - 1 <= T::max_addr()`
        // is a real construction-time invariant but is NOT expressible at the
        // View level today: `Address::max_addr` has no spec counterpart. It is
        // therefore deferred to a future Address spec-interface extension and
        // is not needed by any in-scope getter.)
        self@.wf()
    }
}

impl<T: Address + View<V = int>> TruncatedMemoryRegion<T> {
    pub open spec fn inv(&self) -> bool {
        // Same non-empty geometry, PLUS page alignment of both endpoints —
        // the load-bearing property `frame.rs` (`size / FRAME_SIZE` exact) and
        // the MMIO allocator (frame-base / overlap math) depend on.
        &&& self@.wf()
        &&& self@.is_page_aligned()
    }
}
```

This refines the pre-existing `TruncatedMemoryRegion::inv` (which had only the
two `% spec_page_size() == 0` clauses) by adding `size > 0`. The addition is
sound and caller-faithful: `0` is a multiple of the page size, yet
`frame.rs` computes `size / FRAME_SIZE - 1` and the MMIO allocator treats `size`
as a positive byte extent, so non-emptiness is genuinely relied upon.

---

## Spec Method Specifications (the four target getters)

These are what the spec phase will attach to the in-scope exec functions. Each is
a trivial, declarative projection of the View — exactly what a getter should
promise.

```rust
// MemoryRegion::start(&self) -> T
ensures result@ == self@.start

// MemoryRegion::size(&self) -> usize
ensures result as int == self@.size

// TruncatedMemoryRegion::start(&self) -> PageAligned<T>
ensures result@ == self@.start
// (page-alignment of the result follows from inv(): self@.is_page_aligned())

// TruncatedMemoryRegion::size(&self) -> usize
ensures result as int == self@.size
```

Because the truncated View delegates to the inner region (`self.0@`), the
truncated getters' `self@.start` / `self@.size` are definitionally the inner
region's, so the delegating implementation (`self.0.start()` / `self.0.size()`)
discharges these against the inner getters' contracts with no extra glue.

---

## Spec Transition Functions

The four target functions are read-only — no transition. The module's **only**
state-mutating exec function is `MemoryRegion::set_cache_policy` (out of the
current verification scope; listed here only to keep the abstraction boundary
complete for later phases):

```rust
impl MemoryRegionView {
    pub open spec fn spec_set_cache_policy(self, policy: MmioCachePolicy) -> MemoryRegionView {
        MemoryRegionView { cache_policy: Some(policy), ..self }
    }
}
// future: ensures self@ == old(self)@.spec_set_cache_policy(policy)
```

`..self` carries the frame condition: `start`, `size`, `typ`, `perm` are
unchanged. No transition is defined for the constructors here — they are
out of scope and produce a fresh View rather than transitioning an existing one.

---

## Design Rationale (per field — substitution test)

> *Substitution test: if the implementation were rewritten with a different
> algorithm/layout, would this field still make sense?*

| Field | Why it is caller-observable | Substitution test |
|-------|-----------------------------|-------------------|
| `start: int` | The geometry's low endpoint and the **sole ordering key** (`Ord` compares only `start`). Callers project it to a raw `usize` (overlap bounds) or a frame number (`into_frame_number`). Equals `into_raw_value() as int` by the `Address` contract. | ✅ Any memory-region representation has a base address; expressing it as `int` is layout-independent. |
| `size: int` | The geometry's length; `[start, start+size)` is the region. Callers use it for inclusive-end overlap math and `size / FRAME_SIZE` frame counts. | ✅ Every region has a length, regardless of how it is stored. |
| `typ: MemoryRegionType` | Exposed verbatim by `typ()`; classifies the region (Usable/Reserved/Mmio/Bad). A caller-level tag, not a storage detail. | ✅ Any implementation that distinguishes region kinds carries this tag. |
| `perm: AccessPermission` | Exposed verbatim by `perm()`; the access rights a caller may inspect/enforce. | ✅ Permission is an abstract attribute, independent of layout. |
| `cache_policy: Option<MmioCachePolicy>` | Exposed verbatim by `cache_policy()` and mutated by `set_cache_policy`; observable PWT/PCD behavior for MMIO regions. `Option` faithfully models "unset vs set". | ✅ Caching behavior is a caller-visible attribute, not an internal optimization. |

All five fields pass the substitution test because they each correspond to a
public getter (or, for `cache_policy`, getter + setter) and to a concept the
caller analysis shows callers actually reason about — none is a mirror of an
implementation-private mechanism. The apparent one-to-one correspondence with the
struct fields is incidental: these struct fields *are* the abstract concepts, so
the View is not "leaking" a layout.

**Quality-review checklist**

| Criterion | Verdict |
|-----------|---------|
| Substitution | ✅ every field survives a rewrite (table above). |
| Caller-only | ✅ each field is understandable from a public getter alone. |
| Complete | ✅ every caller-observable concept in the analysis (geometry + all metadata getters) is representable; alignment is captured by `inv()`. |
| Minimal | ✅ `start`/`size` back the in-scope getters; `typ`/`perm`/`cache_policy` back the module's other public getters/setter. No field is unused module-wide. |
| No code-as-spec | ✅ View states *what* (range + tags), not *how* (no `String` length cap, no field order, no storage type `T`). |

---

## Rejected Alternatives

- **`name: Seq<char>` / `name: Seq<u8>` field.** Excluded. The `name()` getter
  exists, but the caller analysis is explicit that **no caller depends on the
  name semantically** — it is purely descriptive metadata used for `Debug`
  formatting. Per spec-design ("where NOT to invest: display/formatting") and the
  minimality criterion, adding it would bloat the View and every spec that
  pattern-matches it without rejecting any bug. If `name()` is ever brought into
  scope, a `name: Seq<char>` field can be added then.

- **Keep `start` as the concrete type `T` (or `PageAligned<T>`).** Rejected.
  Callers never consume the typed address as-is from these getters; they
  immediately extract `into_raw_value`/`into_frame_number` or compare ordering —
  all of which are defined in terms of `T@: int`. Using `int` keeps the View in
  spec world, avoids overflow reasoning, and unifies the two region kinds under
  one View (`PageAligned<T>` and `T` both view as `int`).

- **Separate View types for `MemoryRegion` and `TruncatedMemoryRegion`.**
  Rejected. The truncated region is a thin newtype over `MemoryRegion<PageAligned<T>>`
  with identical observable geometry/metadata; the only difference is the
  alignment guarantee, which belongs in `inv()`, not in a divergent state shape.
  One shared `MemoryRegionView` makes the delegating getters (`self.0.start()`)
  relate to the inner contracts for free.

- **Fold alignment into the View state (e.g., a `bool aligned` field).**
  Rejected. Alignment is a *property* of the geometry (`start % page_size == 0`),
  derivable from `start`/`size`; storing it separately is redundant and could
  desync. It is correctly expressed as an `inv()` predicate
  (`is_page_aligned`), available only on the truncated kind.

- **Encode the range as an explicit `end: int` instead of `size`.** Rejected.
  Callers receive `size()` directly and compute the inclusive end themselves
  (`compute_inclusive_end`, `size / FRAME_SIZE`); `size` is the primitive the API
  exposes, and `end = start + size` is trivially derivable. Storing `size` keeps
  the View aligned with the getter the callers call.

- **Add an in-range invariant `start + size - 1 <= T::max_addr()` to `inv()`
  now.** Deferred, not adopted. It is a genuine construction-time invariant, but
  `Address::max_addr` currently has no spec/`uninterp` counterpart, so it cannot
  be stated abstractly for generic `T`. None of the four in-scope getters need
  it. It should be added once the `Address` spec interface exposes a maximum-
  address spec function.
