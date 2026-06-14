# View Design: bump_allocator

## Abstract Resource

To callers, `FixedSizeBumpAllocator` is a **fixed-capacity, one-shot slot
vendor**: a pool of `capacity` equally-sized, statically-reserved memory slots
that are handed out **at most once each**, in order, until the pool is exhausted.
The only thing that *changes* over the allocator's life is **how many slots have
already been vended**; everything else (where the bytes live, the atomic index,
the stride math, how a byte slot is reinterpreted as `MaybeUninit<T>`) is
mechanism the caller never reasons about.

The single correctness concept callers depend on is therefore: *each successful
allocation yields a slot identity distinct from every prior one, drawn from a
bounded supply.*

---

## View Struct

```rust
pub struct BumpAllocatorView {
    /// Number of slots already vended (the monotonic "bump position").
    /// Equivalently: the index that the next successful allocation will occupy.
    pub allocated: nat,

    /// Maximum number of slots that can ever be vended (== `S::NUM_UNITS`).
    /// Fixed for the life of the allocator.
    pub capacity: nat,
}
```

Two fields, both pure-math (`nat`). No pointers, no `usize`, no atomics, no
stride, no backend handle — those are all implementation surface.

---

## Well-formedness Invariant

```rust
impl BumpAllocatorView {
    /// Abstraction-level well-formedness: the pool never vends more slots than
    /// its capacity. Holds before and after every operation.
    pub open spec fn inv(self) -> bool {
        self.allocated <= self.capacity
    }
}
```

The allocator type's own `inv()` (written in the spec phase) is
`pub open spec fn` and layers the *internal* invariant on top of this:

```rust
impl<const N: usize, const A: usize, S: BssStorage>
    FixedSizeBumpAllocator<N, A, S>
{
    // Public so callers can name the abstract state; closed so the mapping
    // (next_slot atomic -> allocated) does not leak.
    pub closed spec fn view(&self) -> BumpAllocatorView;

    pub open spec fn inv(&self) -> bool {
        &&& self@.inv()
        &&& self@.capacity == S::NUM_UNITS as nat
        // internal_inv() — owned by the spec phase — additionally ties the View
        // to memory: the backing region fits `capacity` slots of stride
        // `align_up(N, A)` within `S::STORAGE_SIZE`, so distinct indices map to
        // disjoint, in-bounds, A-aligned addresses. That linkage is an
        // implementation concern and is NOT part of BumpAllocatorView.
    }
}
```

---

## Spec Transition & Helper Functions

All reusable spec helpers live on the **View type** (per skill rule: no extra
`pub spec fn` on the allocator `impl` beyond `view` and `inv`).

```rust
impl BumpAllocatorView {
    /// True once every slot has been vended; the next request must fail
    /// (`BumpAllocError::Exhausted`). This is the abstract, bidirectional
    /// failure condition for `alloc`.
    pub open spec fn is_exhausted(self) -> bool {
        self.allocated >= self.capacity
    }

    /// Slot index a successful allocation from this state will occupy.
    pub open spec fn next_index(self) -> nat {
        self.allocated
    }

    /// The set of slot indices already vended. Lets a caller state
    /// non-aliasing directly: a fresh allocation's index is NOT in `vended()`.
    pub open spec fn vended(self) -> Set<nat> {
        Set::new(|i: nat| i < self.allocated)
    }

    /// Abstract state after ONE successful allocation: exactly one more slot
    /// consumed. `..self` preserves `capacity` (frame condition).
    pub open spec fn spec_alloc(self) -> BumpAllocatorView {
        BumpAllocatorView { allocated: self.allocated + 1, ..self }
    }
}
```

### How the in-scope functions consume the View (contract sketches for later phases)

These are *not* the deliverable of this phase, but they validate that the View
is complete enough to express every caller expectation.

- **`alloc(&self)`**
  - success ⇒ `!old@.is_exhausted()`; new logical state `== old@.spec_alloc()`;
    returned slot corresponds to index `old@.next_index()`.
  - failure ⇒ `old@.is_exhausted()` (Exhausted) **and** logical state unchanged.
  - *Uniqueness* is derivable, not a separate clause: the returned index
    `old@.allocated ∉ old@.vended()` and lands in `vended()` afterward, so any
    two successful allocations carry distinct indices ⇒ disjoint memory (via the
    type-`inv` index→address mapping). No `exists` needed — the index is the
    deterministic value `old@.allocated`.

- **`alloc_as::<T>(&self)`** — pure type-gate then delegate.
  - `size_of::<T>() != N` ⇒ `Err(SizeMismatch)`, state unchanged.
  - `align_of::<T>() > A` ⇒ `Err(AlignmentMismatch)`, state unchanged.
  - otherwise behaves exactly as `alloc` (same `spec_alloc` transition), with the
    returned `MaybeUninit<T>` sitting at slot index `old@.next_index()`.

- **`align_up(value, alignment)`** — **View-independent** (pure arithmetic, no
  allocator state). Contract is purely mathematical:
  `Some(m) ⇒ alignment != 0 && m % alignment == 0 && m >= value && m - value < alignment`
  (least multiple ≥ `value`); `None ⇒ alignment == 0 || that least multiple
  overflows `usize``. It is mentioned here only because it is in scope; it does
  not touch `BumpAllocatorView`.

- **`as_mut_ptr()` (`BssStorage` trait method)** — also **View-independent**.
  Its guarantees (stable base, ≥ `STORAGE_SIZE` writable bytes, sufficient
  alignment) are a **trait obligation** that backs the allocator type's
  `internal_inv()` index→address mapping. They are external-bottom / trait-level
  trust, not abstract allocator state, so the base pointer is deliberately absent
  from the View.

---

## Design Rationale

### `allocated: nat`
The bump position is the *only* mutable abstract state. Every caller expectation
reduces to it: exhaustion (`allocated >= capacity`), the identity of the next
slot (`= allocated`), and uniqueness (distinct `allocated` values ⇒ distinct
slots). **Substitution test:** a completely different one-shot allocator (free
list, generation counter, segmented pools) still has a well-defined "number of
slots handed out so far." ✅

### `capacity: nat`
The exhaustion threshold the caller relies on for *graceful* failure ("at most
`NUM_UNITS` slots; the next request returns `Err(Exhausted)`"). Keeping it as a
field makes `is_exhausted` / `inv` self-contained instead of reaching into
`S::NUM_UNITS` from spec context, and the allocator `inv()` pins
`capacity == S::NUM_UNITS`. **Substitution test:** any fixed-capacity allocator
has a maximum supply, regardless of how it is stored or counted. ✅

### Why these two suffice (Completeness check)

| Caller-observable concept (from analysis) | Expressed via |
|---|---|
| Uniqueness / no aliasing | distinct `next_index()` per success; `vended()` set membership |
| In-bounds & well-formed slot | index `< capacity` + type-`inv` index→address map |
| Bounded capacity, graceful exhaustion | `is_exhausted()` ⇔ `allocated >= capacity` |
| Type-match gating (`alloc_as`) | direct `N`/`A` comparison in contract (config, see below) |
| Thread-safe handout (no double-vend) | monotonic `spec_alloc` (`allocated + 1`) |
| `'static` stability | type-system + `as_mut_ptr` trait obligation (not View state) |

### Quality review (view-design Step 4)

| Criterion | Result |
|---|---|
| Substitution | Both fields survive a full rewrite. ✅ |
| Caller-only | `allocated`/`capacity` are understandable with zero impl knowledge. ✅ |
| Complete | Every caller-observable concept is representable (table above). ✅ |
| Minimal | Both fields appear in specs (`inv`, `is_exhausted`, `spec_alloc`, `next_index`). ✅ |
| No code-as-spec | Captures *what* (a slot was consumed), not *how* (CAS loop, offsets). ✅ |

---

## Rejected Alternatives

- **`next_slot: usize` / raw bump index.** A direct mirror of the internal
  `AtomicUsize`. Fails substitution (a free-list rewrite has no such index) and
  drags machine types into spec world. Rejected — it *is* the implementation.

- **`base: *mut u8` / `Seq<(addr, len)>` of allocated address ranges.** Encodes
  the concrete memory layout and base pointer. Addresses are produced by
  `as_mut_ptr` + stride math — pure implementation detail that a different
  backend or layout would not share. Disjointness is better expressed
  abstractly via *index injectivity*; the index→address mapping is confined to
  the allocator's `internal_inv()`. Rejected.

- **`unit_size: nat` (N) and `unit_align: nat` (A) as View fields.** These are
  `const` generic *parameters*, not mutable state, and are already nameable
  directly as `N`/`A` (and `size_of`/`align_of`) inside `alloc_as`'s contract.
  Putting them in the View would model configuration as state and violate
  minimality (the value never changes, no transition touches it). The type-match
  gating is expressed by comparing `size_of::<T>()`/`align_of::<T>()` against
  `N`/`A` directly. Rejected as View fields.

- **`stride: nat` (`align_up(N, A)`).** Derived configuration used only to
  compute addresses — an implementation quantity. Lives in `internal_inv()` if
  needed. Rejected.

- **Per-slot initialization / zeroed state (e.g. `Seq<bool>` "is zeroed").**
  Callers do rely on BSS zero-init, but that guarantee originates in the
  `BssStorage` backend, **not** in `alloc`/`alloc_as` (which hand out
  `MaybeUninit`/raw bytes). Modeling it here would over-claim and couple the View
  to a backend property. The View stays silent on byte contents (and must not
  contradict zero-init). Rejected.

- **`free: nat` (remaining slots) instead of `allocated`.** Informationally
  equivalent (`free == capacity - allocated`) but makes the *monotonic* nature of
  the bump position less direct and complicates the `spec_alloc` transition
  (decrement vs. the natural increment). `allocated` reads as "what was handed
  out," matching the uniqueness argument. Rejected in favor of `allocated`.

- **`BumpAllocError` variant as abstract state.** Callers explicitly discard the
  variant after logging (collapse to `OutOfMemory`). The View only needs the
  *condition* `is_exhausted()` for the dominant failure; the remaining variants
  (`Overflow`/`OutOfBounds`/`Misaligned`) are static impossibilities the
  type-`inv` rules out, not abstract states. Rejected.

---

## Notes / Open Modeling Consideration

`alloc`/`alloc_as` take `&self` and mutate through an interior-mutable
`AtomicUsize`, so the logical `old@ -> old@.spec_alloc()` transition is realized
**without** a `&mut self`. The View deliberately models the *logical* effect; how
that effect is published atomically (and stays per-thread unique under
concurrency) is implementation/`internal_inv()` territory and does not change the
abstract state shape above. The spec phase resolves how the contract phrases the
"before/after" relation given `&self` + atomics.
