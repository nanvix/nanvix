# View Design: `bump_allocator` (`src/libs/bump_allocator/src/lib.rs`)

> Phase output. Designs the abstract `View` for `FixedSizeBumpAllocator` (plus the
> supporting numeric spec for `align_up` and the backend spec for `as_mut_ptr`).
> Built **only** from `caller_analysis.md` and the body-removed public API.
>
> Target functions in scope: `FixedSizeBumpAllocator::alloc_as`,
> `FixedSizeBumpAllocator::alloc`, `align_up`, `as_mut_ptr`.

---

## 1. Abstract Resource (recap, from caller analysis)

A **fixed-capacity pool of `capacity` equal-sized slots**, each `unit_size` bytes,
each aligned to `unit_align`, carved at a fixed `stride` from a single statically
reserved region that starts at address `base` and spans `storage_size` bytes. The
allocator is a *monotone consumer* of that pool: it hands out each slot **at most
once** as a unique `'static mut` reference, and reports `Exhausted` once all
`capacity` slots are gone.

What callers actually observe and depend on (the contract the View must support):

1. **Uniqueness / non-aliasing** — every successful allocation is distinct from
   every prior one; no two live slots overlap.
2. **In-bounds** — every returned slot lies fully inside `[base, base + storage_size)`.
3. **Alignment** — every returned slot is `unit_align`-aligned (and `alloc_as`
   additionally needs `align_of::<T>() <= unit_align`).
4. **Monotone capacity** — at most `capacity` successful allocations; the
   `(capacity+1)`-th and beyond return `Exhausted`; consumption never decreases.
5. **No spurious consumption on error** — size/alignment/overflow/bounds faults do
   not consume a slot and do not hand out a usable-but-invalid reference.

The View must make all five expressible **without naming `AtomicUsize`, `Ordering`,
the CAS retry loop, `MaybeUninit`, `PhantomData`, or any other implementation
mechanism** (spec-design: abstract over mechanism, model the observable resource).

---

## 2. The View

The only thing that *changes* at run time is **how many slots have been consumed**.
Everything else (`base`, `stride`, sizes, capacity) is fixed for the lifetime of the
allocator. The View therefore carries the fixed configuration (so that `inv()` and
the per-call specs are self-contained and can talk about bounds/alignment) plus the
single dynamic quantity `allocated`.

```rust
/// Abstract view of a `FixedSizeBumpAllocator<N, A, S>`.
///
/// Pure ghost description of the slot pool. Contains no atomics, no pointers as
/// `*mut`, and no algorithm-specific cursor — only the abstract pool geometry and
/// the count of slots already handed out.
pub ghost struct BumpView {
    /// Base address of the backing region (the integer value of `S::as_mut_ptr()`).
    pub base: int,
    /// Distance in bytes between consecutive slots. Equals `align_up(unit_size, unit_align)`.
    pub stride: nat,
    /// Size of each slot in bytes (the const generic `N`).
    pub unit_size: nat,
    /// Required alignment of each slot in bytes (the const generic `A`).
    pub unit_align: nat,
    /// Total number of slots the pool can ever yield (`S::NUM_UNITS`).
    pub capacity: nat,
    /// Total size in bytes of the backing region (`S::STORAGE_SIZE`).
    pub storage_size: nat,
    /// Number of slots already handed out. The *only* dynamic field.
    pub allocated: nat,
}
```

Attachment: `pub uninterp spec fn bump_view<const N, const A, S: BssStorage>(a: &FixedSizeBumpAllocator<N, A, S>) -> BumpView;`
(a free uninterpreted accessor, used in place of `impl View`/inherent `spec fn view`).
A second `impl` block on `FixedSizeBumpAllocator` alongside the exec-method block crashes
the Verus front end of this `include!`-composed crate with a duplicate-impl-path panic
(`vir/src/context.rs`); the free function is the panic-free analog and is read by callers
exactly like `self.view()`. `base`, `unit_size`, `unit_align`, `capacity`, `storage_size`,
`stride` are pinned by `inv()` to the type-level constants; `allocated` abstracts the
atomic cursor.

### 2.1 Derived spec helpers

```rust
impl BumpView {
    /// Address of slot `i` (0-based). Deterministic geometry of the pool.
    pub open spec fn slot_addr(self, i: int) -> int {
        self.base + i * (self.stride as int)
    }

    /// A slot index is *consumed* iff it is below the high-water mark.
    pub open spec fn is_consumed(self, i: int) -> bool {
        0 <= i < self.allocated
    }

    /// The allocator still has at least one free slot.
    pub open spec fn has_capacity(self) -> bool {
        self.allocated < self.capacity
    }
}
```

`slot_addr` is the bridge between the abstract count and the concrete addresses
callers receive, **without** committing to *how* the address is produced — it is the
pool's geometry, equally valid for any equal-stride pool allocator.

---

## 3. Invariant `inv()`

```rust
impl BumpView {
    pub open spec fn inv(self) -> bool {
        // (a) Geometry is well formed.
        &&& self.unit_size > 0
        &&& self.unit_align > 0
        &&& self.stride >= self.unit_size            // a slot fits in its stride
        // (b) Stride is the up-alignment of the unit size to the unit alignment.
        &&& align_up_spec(self.unit_size, self.unit_align) == Some(self.stride)
        &&& self.stride % self.unit_align == 0       // hence every slot start is A-aligned …
        &&& self.base % (self.unit_align as int) == 0 // … given an A-aligned base (BssStorage duty)
        // (c) The pool fits inside the backing region.
        &&& self.capacity * self.stride <= self.storage_size
        // (d) Addresses do not wrap the usize space.
        &&& self.base >= 0
        &&& self.base + (self.storage_size as int) <= usize::MAX + 1
        // (e) Monotone-capacity bound (cursor never passes the end).
        &&& self.allocated <= self.capacity
    }
}
```

Why each clause exists (caller-driven, not implementation-driven):

| Clause | Guarantees caller invariant |
|--------|-----------------------------|
| (a) `stride >= unit_size > 0` | slots are non-empty and don't overlap their stride → **uniqueness/non-aliasing** |
| (b) `stride == align_up(N,A)`, `stride % A == 0`, `base % A == 0` | `slot_addr(i) % A == 0` for all `i` → **alignment** |
| (c) `capacity * stride <= storage_size` | `slot_addr(i) + N <= base + storage_size` for `i < capacity` → **in-bounds** |
| (d) no-wrap | the address arithmetic in (b)/(c) is sound (no `usize` overflow) → backs `Overflow`/`OutOfBounds` being unreachable on the success path |
| (e) `allocated <= capacity` | **monotone capacity** ceiling; `Exhausted` boundary is exactly `allocated == capacity` |

These are exactly the facts a later proof needs to discharge non-aliasing,
alignment, and in-bounds for *every* returned slot, and they are stated purely over
the abstract pool.

---

## 4. Supporting specs for the other in-scope functions

### 4.1 `align_up` — pure numeric spec (View-independent)

`align_up` is a free `const fn`; it has no allocator state, so it gets a standalone
numeric spec function that the View's `inv()` clause (b) and `stride` reference.

```rust
/// Least multiple of `alignment` that is `>= value`; `None` on `alignment == 0`
/// or when that multiple overflows `usize`.
pub open spec fn align_up_spec(value: nat, alignment: nat) -> Option<nat> {
    if alignment == 0 {
        None
    } else {
        // ceil(value / alignment) * alignment
        let m = ((value + alignment - 1) / alignment) * alignment;
        if m > usize::MAX as nat { None } else { Some(m) }
    }
}
```

Caller-required properties this captures (from caller analysis):
- **total / never panics**, pure, `const`-evaluable;
- `align_up_spec(v,a) == Some(r)` ⇒ `a | r`, `r >= v`, `r < v + a` (least such);
- already-aligned `v` (`v % a == 0`) ⇒ `r == v`;
- `None` **iff** `a == 0` or overflow — the only failure signals callers branch on.

### 4.2 `as_mut_ptr` — backend address spec

`as_mut_ptr` is the `BssStorage` trait method the allocator re-reads on every
allocation. Its abstract meaning is *"reveal the pool base"*: it must return the same
address every call, and that address is exactly `BumpView::base`.

```rust
// Conceptual backend spec (attached to S: BssStorage in a later phase):
//   ensures  result as int == base_of::<S>()          // stability: same value each call
//            base_of::<S>() % (A as int) == 0          // A-aligned base
//            S::STORAGE_SIZE writable bytes from result // (carried as an unsafe/TCB duty)
```

`base_of::<S>()` is the ghost constant the allocator's `view().base` is pinned to.
The "≥ `STORAGE_SIZE` writable, exclusively-owned bytes" portion is the unsafe
`BssStorage` contract and lives in the TCB (it cannot be proven from Rust types); the
View consumes it as an assumption, it does not re-derive it.

---

## 5. Spec transitions of the target functions

State is written `v = self.view()`; `v'` is the post-state. Because `alloc`/`alloc_as`
take `&self` over an interior-mutable atomic, the post-state `v'` is the
*logically next* abstract state observed through the allocator's atomic-ghost token
(modeling detail deferred to the proof phase). All five clauses below are stated only
over `BumpView`, never over the atomic.

### 5.1 `alloc(&self) -> Result<&'static mut [u8; N], BumpAllocError>`

```
requires  v.inv()
ensures   v'.inv()
          // unchanged configuration:
          v'.base == v.base && v'.stride == v.stride && v'.unit_size == v.unit_size
          && v'.unit_align == v.unit_align && v'.capacity == v.capacity
          && v'.storage_size == v.storage_size

          match result {
            Ok(slot) => {
              // a free slot existed and exactly one was consumed
              &&& v.has_capacity()
              &&& v'.allocated == v.allocated + 1
              // the returned reference is the freshly consumed slot k = v.allocated
              &&& slot as int == v.slot_addr(v.allocated as int)
              // contract facts for that slot:
              &&& slot as int % (v.unit_align as int) == 0                     // aligned
              &&& v.base <= slot as int
                  && slot as int + (N as int) <= v.base + v.storage_size       // in-bounds
              // distinct from every previously handed-out slot:
              &&& forall|j: int| 0 <= j < v.allocated
                      ==> slot as int != v.slot_addr(j)
            }
            Err(BumpAllocError::Exhausted) => {
              // pool empty; nothing consumed
              &&& !v.has_capacity()           // v.allocated == v.capacity
              &&& v'.allocated == v.allocated
            }
            Err(_) => {
              // Overflow / OutOfBounds / Misaligned: no consumption observable
              &&& v'.allocated == v.allocated
            }
          }
```

Notes:
- `forall j` distinctness is **derivable** from `inv()` (`stride >= unit_size > 0` makes
  `slot_addr` injective on `0..capacity`), but is surfaced in the ensures because it is
  precisely what the `alloc_returns_distinct_slots` test and the kernel's `unsafe`
  soundness depend on.
- On the non-`Exhausted` error arms, `inv()` clauses (b)–(d) make `Overflow`,
  `OutOfBounds`, and `Misaligned` **unreachable** when `has_capacity()` holds; the
  proof phase can therefore tighten "`Err(_)`" to "`Err(Exhausted)`" once it shows the
  arithmetic cannot fault. The abstract ensures stays conservative: any error leaves
  `allocated` unchanged.

### 5.2 `alloc_as<T>(&self) -> Result<&'static mut MaybeUninit<T>, BumpAllocError>`

`alloc_as` = a typed front door to `alloc`: two compile-time type checks, then the
same pool transition.

```
requires  v.inv()
ensures   v'.inv() && (configuration unchanged, as in 5.1)

          match result {
            // size check fails first → no consumption
            Err(BumpAllocError::SizeMismatch)      => size_of::<T>() != N
                                                       && v'.allocated == v.allocated,
            // then alignment check → no consumption
            Err(BumpAllocError::AlignmentMismatch) => align_of::<T>() > A
                                                       && v'.allocated == v.allocated,
            // otherwise behaves exactly like alloc():
            Ok(r) => {
              &&& size_of::<T>() == N && align_of::<T>() <= A
              &&& v.has_capacity() && v'.allocated == v.allocated + 1
              &&& r as int == v.slot_addr(v.allocated as int)
              &&& r as int % (align_of::<T>() as int) == 0     // ≤ A ⇒ T-aligned
              &&& v.base <= r as int
                  && r as int + (size_of::<T>() as int) <= v.base + v.storage_size
              &&& forall|j: int| 0 <= j < v.allocated ==> r as int != v.slot_addr(j)
            }
            Err(e) => v'.allocated == v.allocated,   // propagated alloc() errors
          }
```

This directly supports the kernel pattern (`alloc_as::<[PteWord; LEN]>()` then
`assume_init_mut()`): the returned `MaybeUninit<T>` is the unique, correctly
sized/aligned, in-bounds slot `k = v.allocated`; failures consume nothing and hand out
nothing.

---

## 6. Substitution test (per field)

> *"If the implementation were rewritten with a different algorithm (e.g. a free-list
> or a watermark recomputed from scratch), would this field still make sense?"*

| Field | Survives? | Reasoning |
|-------|-----------|-----------|
| `base` | ✅ | Any pool allocator draws from a region with a base address; observable via `as_mut_ptr`. Independent of cursor mechanism. |
| `stride` | ✅ | "Distance between equal-sized slots" is a property of the *pool layout*, not the bump algorithm. A free-list over the same region would use the same stride. |
| `unit_size` | ✅ | The `N` contract (`size_of::<T>() == N`) is algorithm-independent. |
| `unit_align` | ✅ | The `A` contract (`align_of::<T>() <= A`, aligned slots) is algorithm-independent. |
| `capacity` | ✅ | "At most `NUM_UNITS` slots" is the resource bound; any fixed-pool allocator has it. |
| `storage_size` | ✅ | The hard region bound `STORAGE_SIZE`; algorithm-independent. |
| `allocated` | ✅ (with care) | Abstract = "number of slots consumed". The module's contract is **no free** (monotone), so a count is the right abstraction. A *different no-free algorithm* (recompute-from-scratch watermark, alternative atomic) yields the same observable count. See §7 for why a count beats `next_slot`. |

Every field passes: the View describes the **pool and its consumption level**, not the
bump cursor. The implementation field `next_slot: AtomicUsize` is *one* way to track
`allocated`; the View commits to none.

---

## 7. Design rationale

- **Abstract the resource, not the mechanism.** The struct names `base/stride/.../allocated`
  — never `AtomicUsize`, `Ordering`, CAS, `MaybeUninit`, or `PhantomData`. Per
  spec-design, callers reason about *unique aligned in-bounds slots*, so that is what
  the View exposes.
- **One dynamic field.** Collapsing the whole atomic protocol to `allocated: nat`
  keeps transitions tiny (`allocated + 1` / unchanged) and makes "no spurious
  consumption" and "monotone capacity" one-liners.
- **Config carried, but pinned.** `base..storage_size` are in the View so `inv()` and
  every per-call ensures are self-contained (bounds/alignment expressible locally),
  yet `inv()` pins them to the type-level constants so they cannot drift.
- **`slot_addr` as the geometry bridge.** A single spec fn turns the abstract count
  into the concrete address callers receive and makes injectivity (hence
  non-aliasing) a pure arithmetic fact derivable from `inv()`.
- **Errors fold to "no consumption".** Every `Err` arm asserts `v'.allocated ==
  v.allocated`; only `Ok` advances. This is the abstract statement of "size/align/
  overflow faults don't burn a slot" and of `Exhausted` leaving the pool untouched.
- **`align_up` stays a free numeric spec.** It owns no allocator state; modeling it on
  the View would be wrong. Instead `inv()` clause (b) *connects* it to `stride`.
- **`as_mut_ptr` reveals `base`, nothing more.** Its stability/alignment/size duties
  are the unsafe `BssStorage` TCB contract; the View consumes them as assumptions and
  binds the returned address to `view().base`.

### Modeling note (deferred to proof phase)
`alloc`/`alloc_as` take `&self` over an interior-mutable atomic, so the `v → v'`
transition is not a literal `&mut` `old(self)`/`self` pair. It will be realized with an
atomic-ghost / `PointsTo` token whose invariant is exactly `BumpView::inv()` and whose
value is `allocated`. This is an attachment detail; the abstract transitions in §5 are
the target the token machinery must satisfy and do not change.

---

## 8. Rejected alternatives

1. **Mirror the implementation: `BumpView { next_slot: usize }`.**
   Rejected. Leaks the cursor mechanism, fails the spirit of the substitution test
   (a recompute-from-scratch or free-list variant has no `next_slot`), and forces
   every spec to re-derive geometry from one scalar. The caller analysis explicitly
   warns: design from the *abstract pool*, not the `AtomicUsize` cursor.

2. **Model consumed slots as a `Set<int>` (or `Seq<int>`) of addresses.**
   Considered for directly expressing uniqueness. Rejected as the *primary* model:
   this allocator never frees and always consumes the lowest free slot, so the set is
   always exactly `{ slot_addr(i) | 0 <= i < allocated }` — fully determined by
   `allocated`. A set adds prover overhead (quantified membership reasoning) for zero
   extra expressiveness here. Uniqueness is instead recovered as a *derived* `forall`
   from `inv()` + `slot_addr` injectivity. (A set *would* be the right call for a
   genuine free-list allocator — noted for substitution awareness.)

3. **Expose raw `*mut u8` / pointer-typed fields in the View.**
   Rejected. The View is ghost; addresses are modeled as `int` (`base`, `slot_addr`).
   Keeping it pointer-free avoids dragging provenance/`PointsTo` mechanics into the
   abstraction boundary and keeps specs about *addresses and bounds*.

4. **Omit configuration; keep only `{ base, allocated }` and fetch `N/A/NUM_UNITS`
   from the type at each use site.**
   Rejected. Forces every ensures and `inv()` to reach for associated consts/const
   generics inline, scattering the geometry and making clauses (b)/(c) hard to read
   and to maintain. Carrying-and-pinning the config localizes all pool facts in one
   self-contained View.

5. **Fold `align_up` into the View (e.g. a `BumpView::align_up` method only).**
   Rejected. `align_up` is a public, View-independent `const fn` with its own callers
   (the kernel uses it in a `const` to compute a stride). It needs a standalone
   numeric spec (`align_up_spec`); the View merely *references* it in `inv()`.

6. **Track richer error state in the View (e.g. last error, fault flags).**
   Rejected. Callers treat errors as transient return values (logged via `Display`,
   mapped to `OutOfMemory`); no error is part of the persistent abstract resource.
   Errors belong in per-call `ensures`, not in `BumpView`.
