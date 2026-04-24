# View Design: `mm::phys::frame`

## Abstract Resource

The frame allocator is a **pool of physical memory frames** — it tracks which
page-sized, page-aligned blocks of physical memory are free and which are
in use (whether dynamically allocated or statically reserved).

To callers, the frame allocator is a set-based resource manager: `alloc()`
removes a frame from the free set, `free()` returns it, and `book()`/
`alloc_range()` permanently reserve frames. The backing data structure
(a sparse bitmap) is invisible to callers.

## Inherited View: `UpoolView`

### What was inherited

The existing `UpoolView` (defined in `mod.spec.rs:50-64`) was introduced
during upstream verification of the `Upool` module. It models `Inner`
(the frame allocator singleton) with two `Set<int>` fields:

```rust
pub struct UpoolView {
    pub allocated_frames: Set<int>,
    pub free_frames: Set<int>,
}
```

With `wf()` requiring page-alignment of all addresses and disjointness.

### Assessment

| Aspect | Verdict | Action |
|--------|---------|--------|
| Two-set model (allocated vs free) | ✅ Correct abstraction | Keep |
| `Set<int>` for frame addresses | ✅ Appropriate (addresses are inherently `int` in spec world) | Keep |
| Disjointness constraint | ✅ Essential invariant | Keep |
| Page-alignment constraint | ✅ Correct | Keep |
| Name `UpoolView` | ❌ Misleading — this is the frame allocator's View, not `Upool`'s | Rename to `FrameAllocView` |
| Field name `allocated_frames` | ⚠️ Comment implied "caller-owned" but `book()` puts MMIO/reserved frames here too | Rename to `allocated`, broaden semantics |
| Missing `addr >= 0` | ❌ Raw `int` allows negative addresses which are nonsensical | Add to `wf()` |
| Missing init postcondition | ❌ No spec captures what `init()` establishes | Address in spec transitions |
| `wf()` is on the View type | ✅ Correct placement | Keep |

### Implementation Note (added during spec phase)

The rename of `UpoolView` → `FrameAllocView` and field renames
(`allocated_frames` → `allocated`, `free_frames` → `free`) were **not
applied** because `UpoolView` is defined in `mod.spec.rs` and shared with
the `upool` and `kpool` modules. Renaming would be a cross-cutting change
affecting other already-verified modules. The existing names are kept for
backward compatibility. The `addr >= 0` constraint was added to
`UpoolView::wf()` as recommended.

## View Struct

```rust
/// Abstract state of the frame allocator singleton.
///
/// Models the allocator as two disjoint sets of page-aligned physical
/// addresses: those currently in use (allocated or reserved) and those
/// available for allocation.
pub struct FrameAllocView {
    /// Frames currently in use — includes both dynamically allocated frames
    /// (returned by `alloc()`) and statically reserved frames (marked by
    /// `book()` / `alloc_range()` during boot).
    pub allocated: Set<int>,
    /// Frames available for allocation via `alloc()`.
    pub free: Set<int>,
}
```

### Field Rationale

| Field | Substitution Test | Caller Need |
|-------|-------------------|-------------|
| `allocated` | Any implementation must track which frames are in use to avoid double-allocation. Survives rewrite. ✅ | `upool.rs` needs to know `alloc()` grants exclusive ownership; `mod.rs` needs `book()` to prevent future allocation. |
| `free` | Any implementation must track which frames are available. Survives rewrite. ✅ | `upool.rs` needs to know when allocation will fail (`free.is_empty()`); `mod.rs` needs `alloc_range()` to verify all frames in a region are free. |

### Why not a single set?

A single `allocated: Set<int>` with free = universe \ allocated would
require a `universe` field to define the complement. The two-set model is
self-contained: each operation's postcondition directly states the new
`allocated` and `free` sets without referencing a third entity. This
matches the existing, well-tested specs.

## Well-formedness Invariant

```rust
impl FrameAllocView {
    /// Well-formedness: all tracked addresses are valid physical frame
    /// addresses (non-negative, page-aligned) and the two sets partition
    /// without overlap.
    pub open spec fn wf(&self) -> bool {
        // Every allocated address is a valid frame address
        &&& forall|addr: int| self.allocated.contains(addr) ==> {
            &&& addr >= 0
            &&& addr % spec_page_size() == 0
        }
        // Every free address is a valid frame address
        &&& forall|addr: int| self.free.contains(addr) ==> {
            &&& addr >= 0
            &&& addr % spec_page_size() == 0
        }
        // Allocated and free are disjoint — a frame cannot be both
        &&& self.allocated.disjoint(self.free)
    }
}
```

### `inv()` on `Inner`

```rust
impl Inner {
    pub open spec fn inv(&self) -> bool {
        &&& self@.wf()
        &&& self.internal_inv()
    }
}
```

The `internal_inv()` connects the abstract View to the concrete bitmap
representation. It is `closed spec` — invisible to callers.

## Spec Transition Functions

These pure spec functions define the abstract state change for each
operation. They are intentionally precondition-free — they compute the
mathematical result. The actual guards (membership, alignment, etc.)
belong in the exec function's `requires`/`ensures` clauses.

```rust
impl FrameAllocView {
    /// Allocate a single frame: move from free to allocated.
    pub open spec fn spec_alloc(self, frame: int) -> FrameAllocView {
        FrameAllocView {
            allocated: self.allocated.insert(frame),
            free: self.free.remove(frame),
        }
    }

    /// Free a single frame: move from allocated to free.
    pub open spec fn spec_free(self, frame: int) -> FrameAllocView {
        FrameAllocView {
            allocated: self.allocated.remove(frame),
            free: self.free.insert(frame),
        }
    }

    /// Reserve (book) a single frame: move from free to allocated.
    /// Semantically identical to alloc at the abstract level, but kept
    /// separate because the operation's intent differs (permanent
    /// reservation vs temporary allocation).
    pub open spec fn spec_book(self, addr: int) -> FrameAllocView {
        FrameAllocView {
            allocated: self.allocated.insert(addr),
            free: self.free.remove(addr),
        }
    }

    /// Reserve a range of frames given as a set.
    pub open spec fn spec_alloc_range(self, frames: Set<int>) -> FrameAllocView {
        FrameAllocView {
            allocated: self.allocated.union(frames),
            free: self.free.difference(frames),
        }
    }
}
```

### Intended Usage in Ensures (Hybrid Style)

Transition functions replace inline `FrameAllocView { ... }` construction
in ensures clauses, while guards remain explicit:

```rust
// Example: alloc ensures (recommended style)
ensures
    self.inv(),
    match result {
        Ok(frame) => {
            &&& frame.inv()
            &&& old(self)@.free.contains(frame@)
            &&& self@ == old(self)@.spec_alloc(frame@)
        },
        Err(_) => {
            &&& self@ == old(self)@
            &&& old(self)@.free.is_empty()
        }
    },
```

### Init Postcondition

`init()` establishes the initial abstract state. Since the frame allocator
is a singleton, `init()` creates the `Inner` value. Its postcondition
should specify:

```rust
// init() establishes:
//   - allocated is empty (no frames in use yet)
//   - free contains all frames defined by the input bitmap
//   - inv() holds
```

The exact `free` set depends on the `SparseBitmap` parameter's abstract
content (which frames the hardware reports as available). This will need
a View/spec for `SparseBitmap` at the trust boundary.

## Quality Review

| Criterion | Status | Notes |
|-----------|--------|-------|
| **Substitution** | ✅ | Both fields survive any rewrite — any allocator must track free vs in-use. |
| **Caller-only** | ✅ | A caller who never reads the implementation understands "allocated" and "free" sets of frame addresses. |
| **Complete** | ✅ | Every caller-observable operation (alloc, free, book, alloc_range, init) can be specified using these two sets. |
| **Minimal** | ✅ | Both fields are used in multiple specs. No field is redundant. |
| **No code-as-spec** | ✅ | The bitmap, frame numbers, sparse regions — all implementation details are hidden. |

## Design Rationale

### Why two sets instead of one set + universe

**Decision**: Keep two sets (`allocated`, `free`), no `universe` field.

A `universe` field would enable a conservation invariant
(`allocated ∪ free == universe`), but:
- No caller currently reasons about "all managed frames" as a set.
- Every operation's postcondition already explicitly constructs the new
  `allocated` and `free` sets, so conservation is implicit.
- Adding `universe` increases proof burden (every transition must show
  the union is preserved) without caller benefit.
- If conservation reasoning becomes necessary (e.g., to prove no frames
  are leaked), `universe` can be added later as a backward-compatible
  extension.

### Why `Set<int>` instead of `Set<usize>`

Physical addresses are `int` in the spec world because:
- `FrameAddress` views to `int` (see `frame.rs:46`).
- `PageAligned<PhysicalAddress>` views to `int`.
- `TruncatedMemoryRegion` views to a struct with `start: int, size: int`.
- Using `int` avoids overflow reasoning in specs.
- The `addr >= 0` constraint in `wf()` ensures non-negativity.

### Why rename from `UpoolView`

The View is for `Inner` (the frame allocator singleton), not for `Upool`
(the user-pool facade). `Upool` has its own View (`UserFrame` views to
`int`). The name `FrameAllocView` directly reflects the module being
specified (`mm::phys::frame`) and avoids confusion with the user pool.

### Why `spec_book` is kept separate from `spec_alloc`

Both produce the same mathematical state transition (move one frame from
free to allocated). However:
- They have different semantic intents: `alloc` is temporary (paired with
  `free`), `book` is permanent (MMIO reservation).
- Separate functions allow separate ensures-clause readability.
- If future callers need to distinguish reserved from allocated frames
  (e.g., by splitting `allocated` into two subsets), only `spec_book`
  would change.

### Why `addr >= 0` was added to `wf()`

The View uses `Set<int>`, which can hold negative values. Physical
addresses are inherently non-negative. Without this constraint, the
abstract state could contain nonsensical negative addresses that no
implementation would produce. This was missing from the inherited
`UpoolView`.

## Rejected Alternatives

### Alternative 1: Single set with universe

```rust
pub struct FrameAllocView {
    pub universe: Set<int>,
    pub allocated: Set<int>,
}
// free = universe \ allocated
```

**Rejected**: Adds a third field and requires every transition to prove
`allocated ⊆ universe`. No caller currently needs this. The two-set
model is simpler and matches the existing, working specs.

### Alternative 2: `Map<int, FrameState>` (enum per frame)

```rust
pub enum FrameState { Free, Allocated, Reserved }
pub struct FrameAllocView {
    pub frames: Map<int, FrameState>,
}
```

**Rejected**: Over-engineering. Callers don't distinguish reserved from
allocated (both are "not free"). The three-state enum adds complexity
without caller benefit. If distinction becomes necessary, it can be
added as a backward-compatible split of the `allocated` set.

### Alternative 3: Sequence-based model

```rust
pub struct FrameAllocView {
    pub frames: Seq<bool>, // indexed by frame number
}
```

**Rejected**: Ties the abstraction to a contiguous, indexed structure —
leaks the bitmap implementation strategy. Set-based model is more
abstract and works regardless of whether the backing store is a bitmap,
a free list, or a buddy allocator.

### Alternative 4: Lifecycle enum wrapping the View

```rust
pub enum FrameAllocState {
    Uninit,
    Ready(FrameAllocView),
}
```

**Rejected for now**: The singleton lifecycle (uninit → initialized) is
enforced by the `INSTANCE_INIT` guard at the module level, not by `Inner`
itself. `Inner` only exists after initialization. The View models
`Inner`'s state, which is always initialized. If module-level specs for
`init()` need a lifecycle model, it can be added at the free-function
spec layer without changing the `FrameAllocView` struct.

### Alternative 5: Adding a `count` or `capacity` field

```rust
pub struct FrameAllocView {
    pub allocated: Set<int>,
    pub free: Set<int>,
    pub total_frames: nat,
}
```

**Rejected**: Derivable from `allocated.len() + free.len()` if needed.
No caller reasons about total capacity. Adding it would create a
redundant proof obligation (keep `total_frames` in sync with set sizes).
